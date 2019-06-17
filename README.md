# repub
markdownをepub 3.0.1形式に変換します

## usage
```
repub 0.1.0
convert markdown(s) to epub

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
    <input>    変換するマークダウンファイル OR 変換するマークダウン文書(複数可)の入ったディレクトリ
```

## zipping
`repub`コマンドによって、`title-repub`フォルダが生成されたとします。
ここから、`sample.epub`ファイルを仕立てる手順は以下のとおりです。

```bash
// title-repubフォルダに移動
cd title-repub

// mimetypeファイルを無圧縮でzipの先頭に
zip -x0q sample.epub mimetype

// OEBPSフォルダをzipに追加
zip -Xr9Dq sample.epub OEBPS/*

// META-INFフォルダをzipに追加
zip -Xr9Dq sample.epub META-INF/*
```

