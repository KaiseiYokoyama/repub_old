# repub
markdownをepub形式に変換します

## usage
```
USAGE:
    repub [OPTIONS] <input>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -i, --bookid <book_id>       Book ID
    -c, --creator <creator>      作者、編集者、翻訳者など
    -l, --language <language>    言語
    -t, --title <title>          タイトルを設定

ARGS:
    <input>    the input file to conver
```

## zipping
`repub`コマンドによって、`title-repub`フォルダが生成されたとします。
ここから、`sample.epub`ファイルを仕立てる手順は以下のとおりです。

```bash
// title-repubフォルダに移動
cd title-repub

// mimetypeファイルをzipの先頭に
zip -x0q sample.epub mimetype

// OEBPSフォルダをzipに追加
zip -Xr9Dq sample.epub OEBPS/*

// META-INFフォルダをzipに追加
zip -Xr9Dq sample.epub META-INF/*
```

