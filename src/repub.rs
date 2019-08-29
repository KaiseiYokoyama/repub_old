use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::{Write, Read};

use rand::Rng;
use rand::distributions::Alphanumeric;
use clap::ArgMatches;
use failure::ResultExt;

/// epubに格納予定のファイル
#[derive(Default, Debug)]
pub struct TmpFiles {
    mimetype: Option<PathBuf>,
    meta_inf: Option<PathBuf>,
    oebps: Option<PathBuf>,
}

#[derive(Debug)]
pub struct RepubBuilder {
    source_file: PathBuf,
    tmp_files: TmpFiles,
    style: Option<PathBuf>,
    title: String,
    creator: String,
    language: String,
    id: String,
    vertical: bool,
    toc_level: u8,
    save_tmp_files: bool,
}

impl Default for RepubBuilder {
    fn default() -> Self {
        RepubBuilder {
            source_file: PathBuf::default(),
            tmp_files: TmpFiles::default(),
            style: Option::default(),
            id: rand::thread_rng().sample_iter(&Alphanumeric).take(30).collect(),
            title: String::default(),
            creator: String::default(),
            language: String::default(),
            vertical: false,
            toc_level: 2,
            save_tmp_files: false,
        }
    }
}

struct Package<'a> {
    metadata: MetaData<'a>,
    items: Items,
}

impl<'a> Package<'a> {
    fn to_opf(&self, vertical: bool) -> String {
        format!(include_str!("literals/package.opf"), &self.metadata.to_xml(), &self.items.to_manifest(), &self.items.to_spine(vertical))
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

        format!(include_str!("literals/package.opf_metadata"),
                &self.title,
                &self.language,
                &self.creator,
                &self.id,
                Utc::now()
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string()
                    .replace("\"", ""))
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

        format!(include_str!("literals/package.opf_manifest"), items)
    }

    fn to_spine(&self, vertical: bool) -> String {
        let mut items = String::new();
        for i in 0..self.items.len() {
            let item = &self.items[i];
            items = format!("{}{}\n", items, item.to_spine(i));
        }

        if vertical {
            // 縦書き->右綴じ
            format!("<spine page-progression-direction=\"rtl\">\n{}\n{}</spine>\n",
                    "<itemref idref=\"navigation\" />",
                    items)
        } else {
            format!("<spine>\n{}\n{}</spine>\n", "<itemref idref=\"navigation\" />", items)
        }
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


trait ToCTrait {
    fn get_inner_items(&mut self) -> &mut Vec<ToCItem>;

    fn get_latest(&mut self, level: u8) -> &mut ToCItem;

    fn push(&mut self, toc_item: ToCItem) {
        self.get_inner_items().push(toc_item);
    }
}

/// 目次の要素のひとつ
#[derive(Debug)]
struct ToCItem {
    is_dummy: bool,
    filename: String,
    id: Option<String>,
    title: String,
    level: u8,
    inner_items: Vec<ToCItem>,
}

impl Default for ToCItem {
    fn default() -> Self {
        ToCItem {
            is_dummy: true,
            filename: String::new(),
            id: None,
            title: String::new(),
            level: 1,
            inner_items: Vec::new(),
        }
    }
}

impl ToCTrait for ToCItem {
    fn get_inner_items(&mut self) -> &mut Vec<ToCItem> {
        &mut self.inner_items
    }

    fn get_latest(&mut self, level: u8) -> &mut ToCItem {
        if level == 1 { return self; }

        let inner_items = self.get_inner_items();
        let toc_item = if inner_items.len() == 0 {
            // initialize
            inner_items.push(ToCItem::default());
            inner_items[0].borrow_mut()
        } else {
            inner_items.last_mut().unwrap()
        };

        toc_item.get_latest(level - 1)
    }
}

impl ToCItem {
    /// xhtml化
    fn to_nav(&self, level: u8) -> String {
        let title = if self.is_dummy {
            String::new()
        } else {
            match &self.id {
                Some(id) => {
                    format!("<a href=\"{}.xhtml#{}\">{}</a>", &self.filename, id, &self.title)
                }
                None => {
                    format!("<span>{}</span>", &self.title)
                }
            }
        };
        let inners: Vec<String> =
            self.inner_items
                .iter()
                .map(|a| a.to_nav(level)).collect();
        let inners_xhtml = if inners.is_empty() {
            String::new()
        } else {
            if self.level >= level {
                format!("<ol hidden=\"hidden\">{}</ol>", inners.join(""))
            } else {
                format!("<ol>{}</ol>", inners.join(""))
            }
        };

        format!("<li>\n{}\n{}\n</li>\n", &title, &inners_xhtml)
    }
}

/// 目次そのもの
#[derive(Default)]
struct ToC {
    inner_items: Vec<ToCItem>
}

impl ToCTrait for ToC {
    fn get_inner_items(&mut self) -> &mut Vec<ToCItem> {
        &mut self.inner_items
    }

    fn get_latest(&mut self, level: u8) -> &mut ToCItem {
        let inner_items = self.get_inner_items();
        let toc_item = if inner_items.len() == 0 {
            // initialize
            inner_items.push(ToCItem::default());
            inner_items[0].borrow_mut()
        } else {
            inner_items.last_mut().unwrap()
        };

        toc_item.get_latest(level)
    }
}

impl ToC {
    fn new(toc_items: Vec<ToCItem>) -> Self {
        let mut origin = ToC::default();

        for toc_item in toc_items {
            let level = toc_item.level;
            origin.push(toc_item, level);
        }

        origin
    }

    fn push(&mut self, toc_item: ToCItem, level: u8) {
        if level == 1 {
            self.inner_items.push(toc_item);
        } else {
            self.get_latest(level - 1).push(toc_item);
        }
    }

    fn to_nav(&self, level: u8, vertical: bool, title: Option<String>) -> String {
        let inners: Vec<String> =
            self.inner_items
                .iter()
                .map(|a| a.to_nav(level)).collect();
        let inners_xhtml = if inners.is_empty() {
            String::new()
        } else {
            inners.join("")
        };
        let title = title.unwrap_or(String::new());
        format!(include_str!("literals/navigation.xhtml"),
                &title,
                if vertical {
                    "<link type=\"text/css\" rel=\"stylesheet\" href=\"styles/vertical.css\" />"
                } else { "" },
                &title,
                &inners_xhtml)
    }
}

impl RepubBuilder {
    /// 絶対パス、あるいは相対パスでソースを指定してRepubBuilderを得る
    pub fn new(path: &Path, matches: &ArgMatches) -> Result<RepubBuilder, failure::Error> {
        // コマンドの実行path
        let origin = &std::env::current_dir()?;

        let md_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            // 指定されたディレクトリへのpath
            origin.join(path)
        };

        // 存在しないpath
        if !md_path.exists() {
            return Err(format_err!("[ERROR] {:?} does not exist.", &md_path));
        }

        // .mdファイルorディレクトリではない
        if md_path.is_file() {
            match md_path.extension() {
                None => {}
                Some(ext) => {
                    if ext != "md" {
                        return Err(format_err!("[ERROR] {:?} is not .md file.", &md_path));
                    }
                }
            }
        }

        let mut repub_builder = RepubBuilder {
            source_file: md_path,
            vertical: matches.is_present("vertical"),
            save_tmp_files: matches.is_present("save_tmp_files"),
            ..RepubBuilder::default()
        };

        // タイトル
        if let Some(title) = matches.value_of("title") {
            repub_builder.titled(title);
        } else {
            print!("Title: ");
            std::io::stdout().flush().context("Failed to read line.")?;

            let mut title = String::new();
            std::io::stdin().read_line(&mut title)
                .expect("Failed to read line");
            repub_builder.titled(title.trim());
        }

        // 作者,編集者,著者
        if let None = matches.value_of("creator") {
            print!("Creator: ");
            std::io::stdout().flush().context("Failed to read line.")?;

            let mut creator = String::new();
            std::io::stdin().read_line(&mut creator)
                .expect("Failed to read line");
            repub_builder.creator(creator.trim());
        }

        // 言語
        if let None = matches.value_of("language") {
            print!("Language: ");
            std::io::stdout().flush().context("Failed to read line.")?;

            let mut language = String::new();
            std::io::stdin().read_line(&mut language)
                .expect("Failed to read line");
            repub_builder.language(language.trim());
        }

        if let Some(id) = matches.value_of("book_id") {
            println!("Book ID: {}", id);
            repub_builder.book_id(id);
        }

        // css style
        if let Some(css) = matches.value_of("style") {
            repub_builder.style(origin.join(css));
        }

        // toc_level
        if let Some(level) = matches.value_of("toc_level") {
            repub_builder.toc_level = match level.parse::<u8>() {
                Ok(ok) => ok - 1,
                Err(_) => {
                    println!("Warning {} は目次のレベルに設定できません", &level);
                    2
                }
            };
        }

        Ok(repub_builder)
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
    fn add_mimetype(&mut self, dir_path: &PathBuf) -> Result<(), failure::Error> {
        // pathを作成
        let mimetype_path = dir_path.join("mimetype");
        // ファイルを作成
        let mut mimetype = File::create(&mimetype_path)?;
        // 書き込み
        mimetype.write_all(include_str!("literals/mimetype").as_bytes())?;

        self.tmp_files.mimetype = Some(mimetype_path);

        Ok(())
    }

    /// META-INFフォルダを配置する
    fn add_meta_inf(&mut self, dir_path: &PathBuf) -> Result<(), failure::Error> {
        // META-INFフォルダのpathを作成
        let meta_inf = dir_path.join("META-INF");
        // フォルダを作成
        std::fs::create_dir_all(&meta_inf)?;

        // container.xmlを作成
        let mut container = File::create(
            meta_inf.join("container.xml"))?;
        // 書き込み
        container.write_all(include_str!("literals/container.xml").as_bytes())?;

        self.tmp_files.meta_inf = Some(meta_inf);

        Ok(())
    }

    /// OEBPSフォルダを設置する
    /// * return - PathBuf of custom.css
    fn add_oebps(&mut self, dir_path: &PathBuf) -> Result<PathBuf, failure::Error> {
        // OEBPSフォルダ設置
        let oebps_path = dir_path.join("OEBPS");
        std::fs::create_dir_all(&oebps_path)?;

        // スタイルフォルダ設置
        let styles = oebps_path.join("styles");
        std::fs::create_dir_all(&styles)?;

        // 縦書きスタイル
        let vertical_css_path = styles.join("vertical.css");
        let mut vertical_css = File::create(vertical_css_path)?;
        vertical_css.write_all(include_str!("literals/vertical.css").as_bytes())?;

        // custom style
        let custom_css_path = styles.join("custom.css");
        File::create(&custom_css_path)?;

        self.tmp_files.oebps = Some(oebps_path);
        Ok(custom_css_path)
    }

    /// .epubファイルを生成する
    /// 生成に失敗したようなら、unzippedなゴミを片付ける
    pub fn build(&mut self) -> Result<(), failure::Error> {
        let res = match self.build_core() {
            // failed
            Err(e) => {
                Err(e)
            }
            // succeeded
            Ok(ok) => {
                Ok(ok)
            }
        };

        if !self.save_tmp_files {
            // ファイル削除
            self.remove_tmp_files();
        }

        res
    }

    /// 一時ファイルを削除する
    fn remove_tmp_files(&self) {
        // pathを変数に代入
        let TmpFiles {
            mimetype, meta_inf, oebps
        } = &self.tmp_files;

        // 存在すれば削除
        // エラーを拾ったときにもゴミ掃除をしたいので、エラー次第ではどれかが存在しないこともありうる
        mimetype.clone().map(|path| std::fs::remove_file(path));
        meta_inf.clone().map(|path| std::fs::remove_dir_all(path));
        oebps.clone().map(|path| std::fs::remove_dir_all(path));
    }

    /// .epubファイルを生成する
    fn build_core(&mut self) -> Result<(), failure::Error> {
        let souce_file_path = self.source_file.clone();
        let dir_path = PathBuf::from(".");

        // mimetypeファイル設置
        self.add_mimetype(&dir_path)?;

        // META-INFフォルダ, container.xmlを設置
        self.add_meta_inf(&dir_path)?;

        // OEBPSフォルダ, styleフォルダ, vertical.css設置
        let custom_css_path = self.add_oebps(&dir_path)?;

        let (mimetype, meta_inf, oebps_path) = match &self.tmp_files {
            TmpFiles {
                mimetype: Some(mimetype),
                meta_inf: Some(meta_inf),
                oebps: Some(oebps_path),
            } => {
                (mimetype, meta_inf, oebps_path)
            }
            _ => {
                return Err(format_err!("[ERROR] file error : {}:{}:{} ",file!(),line!(),column!()));
            }
        };

        // custom.cssに書き込み
        if let Some(path) = &self.style {
            // オリジナルのcssを読み取る
            let mut css = String::new();
            let mut original_css = File::open(path)?;
            original_css.read_to_string(&mut css)?;
            // custom.cssに書き込み
            let mut custom_css = File::create(custom_css_path)?;
            custom_css.write_all(css.as_bytes())?;
        }


        // ファイル読み込み&変換
        let mut items = Items::default();
        let vertical = &self.vertical;
        let mut toc_items = Vec::new();
        if souce_file_path.is_file() {
            convert(&souce_file_path, &oebps_path, &mut items, &mut toc_items, vertical.clone())?;
        } else {
            // ディレクトリから中身一覧を取得
            let mut entries: Vec<_> = std::fs::read_dir(&souce_file_path)?
                .map(|r| r.unwrap())
                .collect();
            // 並べ替え
            entries.sort_by_key(|e| e.path());
            // convert
            for entry in entries {
                let path = entry.path();
                if let Some(ext_os) = path.extension() {
                    if let Some(ext) = ext_os.to_str() {
                        if ext == "md" {
                            convert(&path, &oebps_path, &mut items, &mut toc_items, vertical.clone())?;
                        }
                    }
                }
            }
        }

        // package.opf設置
        let mut package_opf = File::create(
            &oebps_path.join("package.opf"))?;

        // package.opf書き込み準備
        let metadata = MetaData {
            title: &self.title,
            creator: &self.creator,
            language: &self.language,
            id: &self.id,
        };

        // package.opf書き込み
        let package = Package { metadata, items };
        package_opf.write_all(&package.to_opf(self.vertical.clone()).as_bytes())?;

        // navigation.opf作成
        let mut navigation_opf = File::create(
            &oebps_path.join("navigation.xhtml"))?;
        let toc = ToC::new(toc_items);

        navigation_opf.write_all(&toc.to_nav(self.toc_level, self.vertical, Some(String::from("目次"))).as_bytes())?;


        // zip圧縮
        self.make(&mimetype, &meta_inf, &oebps_path)?;
//        self.make_with_command(mimetype, meta_inf, oebps_path)?;

        Ok(())
    }

    /// zip前のフォルダのpathから.epubを生成する
    fn make(&self, mimetype: &PathBuf, meta_inf: &PathBuf, oebps: &PathBuf) -> ZipResult<()> {
        //        use zip::result::ZipResult;
        use zip::write::{FileOptions, ZipWriter};

        let epub_path = PathBuf::from(&format!("{}.epub", &self.title));
        let epub = match File::create(&epub_path) {
            Ok(file) => {
                file
            }
            Err(_) => {
                std::fs::remove_file(&epub_path)?;
                File::create(&epub_path)?
            }
        };

        let mut writer = ZipWriter::new(epub);
        let method = CompressionMethod::Deflated;

        // mimetype
        {
            writer.start_file(mimetype.to_str().unwrap(),
                              FileOptions::default().compression_method(CompressionMethod::Stored))?;
            writer.write(std::fs::read_to_string(mimetype)?.as_bytes())?;
        }

        // META-INF
        writer.add_directory_from_path(meta_inf,
                                       FileOptions::default().compression_method(method))?;

        // inner of META-INF
        for entry in std::fs::read_dir(&meta_inf)? {
            let path = entry?.path();
            if path.is_file() {
                writer.start_file_from_path(path.as_path(),
                                            FileOptions::default().compression_method(method))?;
                writer.write(std::fs::read_to_string(path)?.as_bytes())?;
            }
        }

        // OEBPS
        writer.add_directory_from_path(oebps, FileOptions::default().compression_method(method))?;

        // inner of OEBPS
        for entry in std::fs::read_dir(&oebps)? {
            let path = entry?.path();
            if path.is_file() {
                writer.start_file_from_path(path.as_path(), FileOptions::default())?;
                writer.write(std::fs::read_to_string(path)?.as_bytes())?;
            }
        }

        // styles
        let styles = oebps.join("styles");
        writer.add_directory_from_path(&styles, FileOptions::default().compression_method(method))?;
        for entry in std::fs::read_dir(&styles)? {
            let path = entry?.path();
            if path.is_file() {
                writer.start_file_from_path(path.as_path(), FileOptions::default())?;
                writer.write(std::fs::read_to_string(path)?.as_bytes())?;
            }
        }

        writer.finish()?;

        Ok(())
    }

    /// zip前のフォルダのpathからコマンドを用いて.epubを生成する
    #[allow(dead_code)]
    fn make_with_command(&self, mimetype: &PathBuf, meta_inf: &PathBuf, oebps: &PathBuf) -> Result<(), failure::Error> {
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
        }

        Ok(())
    }
}

use scraper::{Html, Selector};
use zip::CompressionMethod;
use zip::result::ZipResult;
use core::borrow::BorrowMut;

/// domからheaderを読み取り、li要素のVecを返す
fn toc_from_dom(dom: Html, filename: &str) -> Result<Vec<ToCItem>, failure::Error> {
    let header_selector = match Selector::parse("h1,h2,h3,h4,h5") {
        Ok(selector) => selector,
        Err(_) => {
            return Err(format_err!("[ERROR] selector parse error : {}:{}:{} ",file!(),line!(),column!()));
        }
    };
    let headers = dom.select(&header_selector);

    let toc_items: Vec<ToCItem> = headers.map(|header| {
        // header text
        let title = header.text().next().map_or(String::from("UNWRAP ERROR: HEADER TEXT"), |text| text.to_string());
        let level = match header.value().name()[1..].parse::<u8>() {
            Ok(level) => level,
            Err(_) => 6,
        };

        let element_ref = header.select(&Selector::parse("a[id]")
            .expect(&format!("[ERROR] selector parse error : {}:{}:{} ", file!(), line!(), column!())))
            .next();

        match element_ref {
            // idあり -> a要素
            Some(id) => {
                let id = id.value().id().map(|id| id.to_string());
                ToCItem {
                    is_dummy: false,
                    filename: filename.to_string(),
                    id,
                    title,
                    level,
                    ..ToCItem::default()
                }
            }
            // idなし -> span要素
            None => {
                ToCItem {
                    is_dummy: false,
                    filename: filename.to_string(),
                    title,
                    level,
                    ..ToCItem::default()
                }
            }
        }
    }).collect();

    Ok(toc_items)
}

fn convert(source_path: &PathBuf, oebps_path: &PathBuf, items: &mut Items, toc_items: &mut Vec<ToCItem>, vertical: bool) -> Result<(), failure::Error> {
    use comrak::{markdown_to_html, ComrakOptions};

    // source file
    let mut md_file = File::open(&source_path)?;
    // content
    let mut md = String::new();
    md_file.read_to_string(&mut md)?;
    // convert
    let comrak_options = ComrakOptions {
        ext_header_ids: Some("header-".to_string()),
        hardbreaks: true,
        ..ComrakOptions::default()
    };
    let html = format!(include_str!("literals/template.xhtml"),
                       if vertical { "<link type=\"text/css\" rel=\"stylesheet\" href=\"styles/vertical.css\" />" } else { "" }
                       , source_path.file_name().unwrap().to_str().unwrap(), markdown_to_html(&md, &comrak_options));

    // source file name
    let name = source_path.file_stem().unwrap().to_str().unwrap().replace(" ", "_");

    // toc
    let dom = Html::parse_document(&html);
    toc_items.append(&mut toc_from_dom(dom, &name)?);

    // xml path
    let mut xhtml_path = PathBuf::from(name);
    xhtml_path.set_extension("xhtml");
    let xhtml_file_path = &oebps_path.join(&xhtml_path);
    // xml file
    File::create(xhtml_file_path)?.write_all(&html.as_bytes())?;

    items.items.push(
        Item {
            href: xhtml_path.file_name().unwrap().to_str().unwrap().to_string(),
            ..Item::default()
        }
    );

    Ok(())
}
