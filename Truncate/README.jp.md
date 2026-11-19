# Truncate

*[Read in English](README.md) · [Lire en français](README.fr.md) · [阅读中文版](README.zh-CN.md)*

ファイルを思いどおりの正確なサイズにします。作成、拡張（ゼロで埋める）、縮小、または内容のずらし／切り取りができます。依存関係のない純粋な Rust です。[dbat/truncate](https://github.com/dbat/truncate) に着想を得ていますが、元の符号を使った書き方の代わりに、位置引数とオプションによる、よりわかりやすい書き方を採用しています。

## 使い方

```
truncate <ファイル> [サイズ[K|M|G|T]] [--skip N[K|M|G|T]] [--trim-end N[K|M|G|T]] [--quiet]
truncate <ファイル>
```

ファイルが存在せず `サイズ` が指定された場合は作成されます。存在しないファイルを切り詰めようとするとエラーになります。サイズは、K/M/G/T の接尾辞（2 進、×1024、大文字小文字を区別しない）がなければバイト単位です。

| 引数 | 意味 |
| --- | --- |
| `サイズ` | 最終的なサイズ。ちょうどこのサイズになるよう作成／縮小／拡張します。拡張した部分はゼロで埋められます。 |
| `--skip N` | 先頭から取り除くバイト数（残りは先頭へずらされ、ずらした内容より後ろはゼロで埋められます）。既定は 0。 |
| `--trim-end N` | 末尾から取り除くバイト数。`サイズ` を指定しない場合のみ有効です。 |
| `--quiet`、`-q` | 変更前後のサイズの要約を表示しません。 |
| `--help`、`-h` | 使い方を表示します。 |

ファイルだけを指定して呼び出すと、`truncate` は何も変更せず、ファイルの現在のサイズを表示します。

## 例

```
truncate foo.bin                          # 現在のサイズを表示するだけ
truncate zero.bin 0                       # ファイルを空にする（必要なら作成）
truncate foo.bin 2G                       # ちょうど 2 GiB で作成／サイズ変更
truncate bar.bin --skip 19K --trim-end 19K # 両端から 19 KiB ずつ切り取る
truncate baz.bin 1T --skip 777            # 777 バイト目から内容をずらし、1 TiB までゼロで埋める
```

サイズを変更するたびに、`--quiet` を指定しない限り要約が表示されます：

```
> truncate foo.bin 2G
foo.bin: 10.00 MiB (10485760 B) -> 2.00 GiB (2147483648 B)
```

## メニュー

`Truncate-Menu.bat` は、この exe を 2 か国語（フランス語／英語）の対話式メニューで包みます。`input\` にファイルを置き、モードを選ぶと、結果が `output\` に出力されます（`input\` の元のファイルは決して変更されません。サイズの表示は、コピーせずにその場で読み取ります）。オプションの書き方を覚えたくない場合に便利です。

## 更新

`Truncate` フォルダーから `truncate_updater.bat` を実行してください。最新リリースをダウンロードして、その場に展開します。`input\` と `output\` フォルダーはそのまま残ります。

## ライセンス

個人的かつ非商用の利用に限り無料です。Copyright (C) 2026 Nyaldee, all rights reserved. 条件の全文は [LICENSE](../LICENSE) をご覧ください。ダウンロードは、このリポジトリの [Releases](https://github.com/Nyaldee/nyaruhodo/releases) からのみ行ってください。
