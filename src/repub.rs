use std::fs::File;
use std::path::{Path, PathBuf};
use rand::Rng;
use rand::distributions::Alphanumeric;
use clap::{App, ArgMatches};
use std::io::{Write, Read};

#[derive(Debug)]
pub struct RepubBuilder {
    source_file: PathBuf,
    style: Option<PathBuf>,
    title: String,
    creator: String,
    language: String,
    id: String,
    vertical: bool,
}

impl Default for RepubBuilder {
    fn default() -> Self {
        RepubBuilder {
            source_file: PathBuf::default(),
            style: Option::default(),
            id: rand::thread_rng().sample_iter(&Alphanumeric).take(30).collect(),
            title: String::default(),
            creator: String::default(),
            language: String::default(),
            vertical: false,
        }
    }
}

struct Package<'a> {
    metadata: MetaData<'a>,
    items: Items,
}

impl<'a> Package<'a> {
    fn to_opf(&self, vertical: bool) -> String {
        format!("<?xml version='1.0' encoding='utf-8'?>\n\
<package unique-identifier=\"BookId\" version=\"3.0\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns=\"http://www.idpf.org/2007/opf\">\n\
{}{}{}\
</package>", &self.metadata.to_xml(), &self.items.to_manifest(), &self.items.to_spine(vertical))
    }
}

struct MetaData<'a> {
    title: &'a str,
    creator: &'a str,
    language: &'a str,
    id: &'a str,
}

impl<'a> MetaData<'a> {
    fn to_xml(&self) -> String {
        use chrono::prelude::*;

        return format!("<metadata>\n\
<dc:title>{}</dc:title>\n\
<dc:language>{}</dc:language>\n\
<dc:creator>{}</dc:creator>\n\
<dc:identifier id=\"BookId\">{}</dc:identifier>\n\
<meta property=\"dcterms:modified\">{}</meta>\n\
</metadata>\n", &self.title, &self.language, &self.creator, &self.id, Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string().replace("\"", ""));
    }
}

#[derive(Default)]
struct Items {
    items: Vec<Item>
}

impl Items {
    fn to_manifest(&self) -> String {
        let mut items = String::new();
        for i in 0..self.items.len() {
            let item = &self.items[i];
            items = format!("{}{}\n", items, item.to_manifest(i));
        }

        return format!("<manifest>\n{}\n{}\n{}\n{}\n</manifest>",
                       "<item id=\"navigation\" href=\"navigation.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\" />",
                       items,
                       "<item id=\"vertical_css\" href=\"styles/vertical.css\" media-type=\"text/css\"/>",
                       "<item id=\"custom_css\" href=\"styles/custom.css\" media-type=\"text/css\"/>");
    }

    fn to_spine(&self, vertical: bool) -> String {
        let mut items = String::new();
        for i in 0..self.items.len() {
            let item = &self.items[i];
            items = format!("{}{}\n", items, item.to_spine(i));
        }

        return if vertical {
            // 縦書き->右綴じ
            format!("<spine page-progression-direction=\"rtl\">\n{}\n{}</spine>\n",
                    "<itemref idref=\"navigation\" />",
                    items)
        } else {
            format!("<spine>\n{}\n{}</spine>\n", "<itemref idref=\"navigation\" />", items)
        };
    }
}

struct Item {
    href: String,
    media_type: String,
}

impl Default for Item {
    fn default() -> Self {
        Item {
            href: "".to_string(),
            media_type: "application/xhtml+xml".to_string(),
        }
    }
}

impl Item {
    /// package.opf内のmanifest要素に変換
    fn to_manifest(&self, id: usize) -> String {
        format!("<item id=\"book_{}\" href=\"{}\" media-type=\"{}\" />",
                id, &self.href, &self.media_type)
    }

    /// package.opf内のspine要素に変換
    fn to_spine(&self, id: usize) -> String {
        format!("<itemref idref=\"book_{}\" />", id)
    }
}

impl RepubBuilder {
    /// 絶対パス、あるいは相対パスでソースを指定してRepubBuilderを得る
    pub fn new(path: &Path, matches: &ArgMatches) -> Result<RepubBuilder, ()> {
        // 指定されたpathが絶対パスであったとき
        let file_path;
        // コマンドの実行path
        let origin = &std::env::current_dir().unwrap();

        if path.is_absolute() {
            file_path = path.to_path_buf();
        } else {
            // 指定されたディレクトリへのpath
            file_path = origin.join(path);
        }

        // 存在しないpath
        if !file_path.exists() {
            println!("[ERROR] {:?} does not exist.", &file_path);
            return Err(());
        }

        // .mdファイルorディレクトリではない
        if file_path.is_file() {
            match file_path.extension() {
                None => {}
                Some(ext) => {
                    if ext != "md" {
                        println!("[ERROR] {:?} is not.md file.", &file_path);
                        return Err(());
                    }
                }
            }
        }

        let mut repub_builder = RepubBuilder {
            source_file: file_path,
            vertical: matches.is_present("vertical"),
            ..RepubBuilder::default()
        };

        // タイトル
        if let Some(title) = matches.value_of("title") {
            repub_builder.titled(title);
        } else {
            print!("Title: ");
            std::io::stdout().flush().unwrap();

            let mut title = String::new();
            std::io::stdin().read_line(&mut title)
                .expect("Failed to read line");
            repub_builder.titled(title.trim());
        }

        // 作者,編集者,著者
        if let None = matches.value_of("creator") {
            print!("Creator: ");
            std::io::stdout().flush().unwrap();

            let mut creator = String::new();
            std::io::stdin().read_line(&mut creator)
                .expect("Failed to read line");
            repub_builder.creator(creator.trim());
        }

        // 言語
        if let None = matches.value_of("language") {
            print!("Language: ");
            std::io::stdout().flush().unwrap();

            let mut language = String::new();
            std::io::stdin().read_line(&mut language)
                .expect("Failed to read line");
            repub_builder.language(language.trim());
        }

        // css style
        if let Some(css) = matches.value_of("style") {
            repub_builder.style(origin.join(css));
        }

        return Ok(repub_builder);
    }

    pub fn titled(&mut self, title: &str) -> &mut Self {
        self.title = title.to_string();
        self
    }

    pub fn creator(&mut self, creator: &str) -> &mut Self {
        self.creator = creator.to_string();
        self
    }

    pub fn language(&mut self, language: &str) -> &mut Self {
        self.language = language.to_string();
        self
    }

    pub fn style(&mut self, style: PathBuf) -> &mut Self {
        self.style = Some(style);
        self
    }

    pub fn book_id(&mut self, book_id: &str) -> &mut Self {
        self.id = book_id.to_string();
        self
    }

    /// mimetypeファイルを配置する
    fn add_mimetype(&self, dir_path: &PathBuf) -> PathBuf {
        // pathを作成
        let mimetype_path = dir_path.join("mimetype");
        // ファイルを作成
        let mut mimetype = File::create(&mimetype_path).unwrap();
        // 書き込み
        mimetype.write_all("application/epub+zip".as_bytes()).unwrap();

        return mimetype_path;
    }

    /// META-INFフォルダを配置する
    fn add_meta_inf(&self, dir_path: &PathBuf) -> PathBuf {
        // META-INFフォルダのpathを作成
        let meta_inf = dir_path.join("META-INF");
        // フォルダを作成
        std::fs::create_dir_all(&meta_inf);

        // container.xmlを作成
        let mut container = File::create(
            meta_inf.join("container.xml")).unwrap();
        // 書き込み
        container.write_all("<?xml version =\"1.0\" ?>\n\
<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\n\
  <rootfiles>\n\
    <rootfile full-path=\"OEBPS/package.opf\" media-type=\"application/oebps-package+xml\" />\n\
  </rootfiles>\n\
</container>".as_bytes());

        return meta_inf;
    }

    /// OEBPSフォルダを設置する
    fn add_oebps(&self, dir_path: &PathBuf) -> (PathBuf, PathBuf) {
        // OEBPSフォルダ設置
        let oebps_path = dir_path.join("OEBPS");
        std::fs::create_dir_all(&oebps_path);

        // スタイルフォルダ設置
        let styles = oebps_path.join("styles");
        std::fs::create_dir_all(&styles);

        // 縦書きスタイル
        let vertical_css_path = styles.join("vertical.css");
        let mut vertical_css = File::create(vertical_css_path).unwrap();
        vertical_css.write_all("html { writing-mode: vertical-rl; }".as_bytes());

        // custom style
        let custom_css_path = styles.join("custom.css");
        File::create(&custom_css_path).unwrap();

        return (oebps_path, custom_css_path);
    }

    pub fn build(&self) {
        let souce_file_path = &self.source_file;
        let mut dir_path = PathBuf::from(".");

        // mimetypeファイル設置
        let mimetype_path = self.add_mimetype(&dir_path);

        // META-INFフォルダ, container.xmlを設置
        let meta_inf = self.add_meta_inf(&dir_path);

        // OEBPSフォルダ, styleフォルダ, vertical.css設置
        let (oebps_path, custom_css_path) = self.add_oebps(&dir_path);

        // custom.cssに書き込み
        if let Some(path) = &self.style {
            // オリジナルのcssを読み取る
            let mut css = String::new();
            let mut original_css = File::open(path).unwrap();
            original_css.read_to_string(&mut css);
            // custom.cssに書き込み
            let mut custom_css = File::create(custom_css_path).unwrap();
            custom_css.write_all(css.as_bytes());
        }

        // package.opf設置
        let mut package_opf = File::create(
            &oebps_path.join("package.opf")).unwrap();

        // package.opf書き込み準備
        let metadata = MetaData {
            title: &self.title,
            creator: &self.creator,
            language: &self.language,
            id: &self.id,
        };
        let mut items = Items::default();

        // ファイル読み込み&変換
        let vertical = &self.vertical;
        let mut lis = Vec::new();
        if souce_file_path.is_file() {
            convert(souce_file_path, &oebps_path, &mut items, &mut lis, vertical.clone());
        } else {
            // ディレクトリから中身一覧を取得
            let mut entries: Vec<_> = std::fs::read_dir(souce_file_path)
                .unwrap()
                .map(|r| r.unwrap())
                .collect();
            // 並べ替え
            entries.sort_by_key(|e| e.path());
            // convert
            for entry in entries {
                let path = entry.path();
                if "md" == path.extension().unwrap().to_str().unwrap() {
                    convert(&path, &oebps_path, &mut items, &mut lis, vertical.clone());
                }
            }
        }

        // package.opf書き込み
        let package = Package { metadata, items };
        package_opf.write_all(&package.to_opf(self.vertical.clone()).as_bytes());

        // navigation.opf作成
        let mut navigation_opf = File::create(
            &oebps_path.join("navigation.xhtml")).unwrap();
        let mut lis_html = String::new();
        for li in lis {
            lis_html = format!("{}{}", lis_html, li);
        }

        navigation_opf.write_all(&format!("<?xml version='1.0' encoding='utf-8'?>\n\
<!DOCTYPE html>\n\
<html xml:lang=\"ja\" lang=\"ja\" xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\n\
<head>\n\
<meta charset=\"utf-8\" />\n\
<title>目次</title>\n\
{}\n\
</head>\n\
<body>\n\
<nav epub:type=\"toc\">\n\
<h1>目次</h1>\n\
<ol>{}</ol>\n\
</nav>\n\
</body>\n\
</html>", if self.vertical.clone() { "<link type=\"text/css\" rel=\"stylesheet\" href=\"styles/vertical.css\" />" } else { "" }, &lis_html).as_bytes());

        // mimetypeファイルの場所(相対パス)
        let mimetype_path = dir_path.join("mimetype");
        // meta_infフォルダの場所(相対パス)
        let meta_inf_path = dir_path.join("META-INF");
        // OEBPSフォルダの場所(相対パス)
        let oebps_path = dir_path.join("OEBPS");

//        self.make(dir_path.as_path(), &mimetype_path, &meta_inf, &oebps_path);
        self.make_with_command(dir_path.as_path(), &mimetype_path, &meta_inf, &oebps_path);
    }

    /// zip前のフォルダのpathから.epubを生成する
    fn make(&self, dir_path: &Path, mimetype: &PathBuf, meta_inf: &PathBuf, oebps: &PathBuf) -> ZipResult<()> {
        use std::io::{Seek, Write};
        use zip::result::ZipResult;
        use zip::write::{FileOptions, ZipWriter};

        let epub_path = format!("{}.epub", &self.title);
        let mut epub_path = dir_path.join(&epub_path);
        let mut epub = match File::create(&epub_path) {
            Ok(file) => {
                file
            }
            Err(_) => {
                std::fs::remove_file(&epub_path);
                File::create(&epub_path).unwrap()
            }
        };

        let mut writer = ZipWriter::new(epub);
        writer.start_file(mimetype.to_str().unwrap(),
                          FileOptions::default().compression_method(CompressionMethod::Stored))?;
        let method = CompressionMethod::Deflated;
        // META-INF
        writer.add_directory_from_path(meta_inf,
                                       FileOptions::default().compression_method(method))?;
        for entry in std::fs::read_dir(&meta_inf).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() {
                writer.start_file_from_path(path.as_path(),
                                            FileOptions::default().compression_method(method));
            }
        }
        // OEBPS
        writer.add_directory_from_path(oebps, FileOptions::default().compression_method(method))?;
        for entry in std::fs::read_dir(&oebps).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() {
                writer.start_file_from_path(path.as_path(), FileOptions::default());
            }
        }

        writer.finish()?;

        return Ok(());
    }

    /// zip前のフォルダのpathからコマンドを用いて.epubを生成する
    fn make_with_command(&self, dir_path: &Path, mimetype: &PathBuf, meta_inf: &PathBuf, oebps: &PathBuf) {
        use std::process::Command;

        if cfg!(target_os = "macos") {
            let epubname = &format!("{}.epub", &self.title);
            Command::new("zip")
                .arg("-x0q")
                .arg(epubname)
                .arg(mimetype.to_str().unwrap())
                .output().expect("Missed zip mimetype");
            Command::new("zip")
                .arg("-Xr9Dq")
                .arg(epubname)
                .arg(meta_inf.to_str().unwrap())
                .output().expect("Missed zip META-INF");
            Command::new("zip")
                .arg("-Xr9Dq")
                .arg(epubname)
                .arg(oebps.to_str().unwrap())
                .output().expect("Missed zip OEBPS");

            // delete files
            std::fs::remove_file(&mimetype.as_path());
            std::fs::remove_dir_all(&meta_inf.as_path());
            std::fs::remove_dir_all(&oebps.as_path());
        }
    }
}

use scraper::{Html, Selector};
use scraper::node::Element;
use std::collections::HashSet;
use std::error::Error;
use zip::CompressionMethod;
use zip::result::ZipResult;

/// domからheaderを読み取り、li要素のVecを返す
fn toc_from_dom(dom: Html, filename: &str) -> Vec<String> {
    let header_selector = Selector::parse("h1,h2,h3").unwrap();
    let headers = dom.select(&header_selector);

    let mut lis: Vec<String> = Vec::new();
    for header in headers {
        // header text
        let text = header.text().next().unwrap_or("UNWRAP ERROR: HEADER TEXT");
        // idの有無を確認
        let li = match header.select(&Selector::parse("a[id]").unwrap()).next() {
            // idあり -> a要素
            Some(id) => {
                format!("<li header=\"{}\"><a href=\"{}.xhtml#{}\">{}</a></li>",
                        header.value().name(),
                        filename,
                        id.value().id().unwrap_or("UNWRAP ERROR: HEADER ID"),
                        text)
            }
            // idなし -> span要素
            None => {
                format!("<li header=\"{}\"><span>{}</span></li>", header.value().name(), text)
            }
        };
        lis.push(li);
    }

    return lis;
}

fn convert(source_path: &PathBuf, oebps_path: &PathBuf, items: &mut Items, lis: &mut Vec<String>, vertical: bool) {
    use comrak::{markdown_to_html, ComrakOptions};

    // source file
    let mut md_file = File::open(&source_path).unwrap();
    // content
    let mut md = String::new();
    md_file.read_to_string(&mut md);
    // convert
    let mut comrak_options = ComrakOptions {
        ext_header_ids: Some("header-".to_string()),
        hardbreaks: true,
        ..ComrakOptions::default()
    };
    let html = format!("<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\n\
<head>\n\
<meta charset=\"utf-8\" />\n\
{}\n\
{}\n\
<title>{}</title>\n\
</head>\n\
<body>\n{}\n</body>\n</html>",
                       if vertical { "<link type=\"text/css\" rel=\"stylesheet\" href=\"styles/vertical.css\" />" } else { "" }
                       , "<link type=\"text/css\" rel=\"stylesheet\" href=\"styles/custom.css\" />"
                       , source_path.file_name().unwrap().to_str().unwrap(), markdown_to_html(&md, &comrak_options));

    let xhtml = format!("<?xml version='1.0' encoding='utf-8'?>\n\
<!DOCTYPE html>\n\
{}
", &html);

    // source file name
    let name = source_path.file_stem().unwrap().to_str().unwrap().replace(" ", "_");

    // toc
    let dom = Html::parse_document(&html);
    lis.append(&mut toc_from_dom(dom, &name));

    // xml path
    let mut xhtml_path = PathBuf::from(name);
    xhtml_path.set_extension("xhtml");
    let mut xhtml_file_path = &oebps_path.join(&xhtml_path);
    // xml file
    File::create(xhtml_file_path).unwrap().write_all(&html.as_bytes());

    items.items.push(
        Item {
            href: xhtml_path.file_name().unwrap().to_str().unwrap().to_string(),
            ..Item::default()
        }
    )
}
