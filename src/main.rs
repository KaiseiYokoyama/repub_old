use std::path::Path;

mod repub;

#[macro_use]
extern crate clap;

fn main() -> Result<(), ()> {
    use clap::{App, Arg, SubCommand};
    let app = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        // .mdファイルorフォルダ
        .arg(Arg::from_usage("<input> '変換するマークダウンファイル OR 変換するマークダウン文書(複数可)の入ったディレクトリ'"))
        // タイトル
        .arg(Arg::with_name("title")
            .help("タイトルを設定")
            .short("t")
            .long("title")
            .takes_value(true))
        // 著者
        .arg(Arg::with_name("creator")
            .help("作者、編集者、翻訳者など")
            .short("c")
            .long("creator")
            .takes_value(true))
        // 言語
        .arg(Arg::with_name("language")
            .help("言語")
            .short("l")
            .long("language")
            .takes_value(true))
        // id
        .arg(Arg::with_name("book_id")
            .help("Book ID")
            .short("id")
            .long("bookid")
            .takes_value(true))
        // 縦書き
        .arg(Arg::with_name("vertical")
            .help("縦書き")
            .short("v")
            .long("vertical"))
        ;

    let matches = app.get_matches();

    let mut repub_builder = repub::RepubBuilder::new(
        Path::new(&matches.value_of("input").unwrap()), &matches)?;

    println!("{:?}", &repub_builder);

    repub_builder.build();

    return Ok(());
}
