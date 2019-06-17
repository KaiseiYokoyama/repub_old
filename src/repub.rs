use std::fs::File;
use std::path::{Path, PathBuf};
use rand::Rng;
use rand::distributions::Alphanumeric;
use clap::{App, ArgMatches};
use std::io::{Write, Read};

#[derive(Debug)]
pub struct RepubBuilder {
    source_file: PathBuf,
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

        return format!("<manifest>\n{}\n{}\n{}\n</manifest>",
                       "<item id=\"navigation\" href=\"navigation.opf\" media-type=\"application/xhtml+xml\" properties=\"nav\" />",
                       items,
                       "<item id=\"vertical_css\" href=\"styles/vertical.css\" media-type=\"text/css\"/>");
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
        if path.is_absolute() {
            file_path = path.to_path_buf();
        } else {
            // コマンドの実行path
            let origin = &std::env::current_dir().unwrap();
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

    pub fn book_id(&mut self, book_id: &str) -> &mut Self {
        self.id = book_id.to_string();
        self
    }

    pub fn build(&self) {
        let souce_file_path = &self.source_file;
        let mut dir_path = souce_file_path.parent().unwrap()
            .join(&format!("{}{}", &self.title, "-repub"));

        // unzipされたファイルの一時置き場所
        std::fs::create_dir_all(&dir_path);

        // mimetypeファイル設置
        let mut mimetype = File::create(&dir_path.join("mimetype")).unwrap();
        mimetype.write_all("application/epub+zip".as_bytes()).unwrap();

        // META-INFフォルダ設置
        std::fs::create_dir_all(&dir_path.join("META-INF"));

        // container.xml設置
        let mut container = File::create(
            &dir_path.join("META-INF")
                .join("container.xml")).unwrap();
        container.write_all("<?xml version =\"1.0\" ?>\
<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\
  <rootfiles>\
    <rootfile full-path=\"OEBPS/package.opf\" media-type=\"application/oebps-package+xml\" />\
  </rootfiles>\
</container>".as_bytes());

        // OEBPSフォルダ設置
        let oebps_path = &dir_path.join("OEBPS");
        std::fs::create_dir_all(&oebps_path);

        // スタイルフォルダ設置
        let styles = oebps_path.join("styles");
        std::fs::create_dir_all(&styles);

        // 縦書きスタイル
        let vertical_css_path = styles.join("vertical.css");
        let mut vertical_css = File::create(vertical_css_path).unwrap();
        vertical_css.write_all("html { writing-mode: vertical-rl; }".as_bytes());

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
            convert(souce_file_path, oebps_path, &mut items, &mut lis, vertical.clone());
        } else {
            for entry in std::fs::read_dir(souce_file_path).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if "md" == path.extension().unwrap().to_str().unwrap() {
                    convert(&path, oebps_path, &mut items, &mut lis, vertical.clone());
                }
            }
        }

        // package.ops書き込み
        let package = Package { metadata, items };
        package_opf.write_all(&package.to_opf(self.vertical.clone()).as_bytes());

        // navigation.opf作成
        let mut navigation_opf = File::create(
            &oebps_path.join("navigation.opf")).unwrap();
        let mut lis_html = String::new();
        for li in lis {
            lis_html = format!("{}{}", lis_html, li);
        }

        navigation_opf.write_all(&format!("<?xml version='1.0' encoding='utf-8'?>\
<!DOCTYPE html>\
<html xml:lang=\"ja\" lang=\"ja\" xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\
<head>\
<meta charset=\"utf-8\" />\
<title>目次</title>\
</head>\
<body>\
<nav epub:type=\"toc\">\
<h1>目次</h1>\
<ol>{}</ol>\
</nav>\
</body>\
</html>", &lis_html).as_bytes());
    }
}

use scraper::{Html, Selector};
use scraper::node::Element;
use std::collections::HashSet;

// domからheaderを読み取り、headerにidをつけ、headerへのリンクを含むli要素のVecを返す
fn toc_from_dom(dom: Html, filename: &str) -> Vec<String> {
    let header_selector = Selector::parse("h1,h2,h3").unwrap();
    let headers = dom.select(&header_selector);

    let mut lis: Vec<String> = Vec::new();
    for header in headers {
        let li = format!("<li header=\"{}\">{}</li>", header.value().name(), header.inner_html());
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
    let html = format!("<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\n\
<head>\n\
<meta charset=\"utf-8\" />\n\
{}\n\
<title>{}</title>\n\
</head>\n\
<body>\n{}\n</body>\n</html>",
                       if vertical { "<link type=\"text/css\" rel=\"stylesheet\" href=\"styles/vertical.css\" />" } else { "" }
                       , source_path.file_name().unwrap().to_str().unwrap(), markdown_to_html(&md, &ComrakOptions::default()));

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