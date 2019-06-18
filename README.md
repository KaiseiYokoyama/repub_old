# repub
markdownをepub 3.0.1形式に変換します

## example
- convert `.md` file to `.epub`
```bash
repub markdown.md
```

- convert `.md` files in directory to `.epub`
```bash
repub markdown_directory
```

- convert with `.css` file
```bash
repub -s custom.css markdown_directory
```

## usage
```
repub 0.1.0
convert markdown(s) to epub

USAGE:
    repub [FLAGS] [OPTIONS] <input>

FLAGS:
    -h, --help        Prints help information
    -V, --version     Prints version information
    -v, --vertical    縦書き

OPTIONS:
    -i, --bookid <book_id>       Book ID
    -c, --creator <creator>      作者、編集者、翻訳者など
    -l, --language <language>    言語
    -s, --css <style>            cssを指定
    -t, --title <title>          タイトルを設定

ARGS:
    <input>    変換するマークダウンファイル OR 変換するマークダウン文書(複数可)の入ったディレクトリ
```

## zipping
MacOSでは、プログラムがzipコマンドを実行して`.epub` ファイルを生成します。
Windows環境ではプログラムによる`.epub`ファイルの生成は行われませんので、`epubpack`などを使用してください。
