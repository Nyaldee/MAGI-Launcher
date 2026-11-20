# Silent CMD

*[Read in English](README.md) · [Lire en français](README.fr.md) · [阅读中文版](README.zh-CN.md)*

コマンドやスクリプト（`.bat`／`.cmd`）を、コンソールウィンドウを表示せずに実行する Windows 用の小さなランチャーです。

## 使い方

```
silent_cmd.exe [オプション] <コマンド> [引数...]
```

オプションの後に続くものはすべて、引用符や特殊文字（`&`、`^`、`%`…）も含めて、書かれたとおりに `cmd /C` へ渡されます。コマンドの終了コードや出力のリダイレクトは通常どおり機能し、隠れるのはコンソールウィンドウだけです。オプションはコマンドの前に、任意の順序で組み合わせられます。

| オプション | 説明 |
|---|---|
| `--delay <秒>` | コマンドを実行する前に待機します。 |
| `--log <ファイル>` | コマンドの出力とエラーを `ファイル` に書き込みます（実行のたびに上書き）。相対パスは、`--dir` が指定されていればそこを基準にします。 |
| `--dir <パス>` | `パス` を作業フォルダーとしてコマンドを実行します。 |
| `--nowait` | コマンドの終了を待たず、起動したらすぐに戻ります。 |

```
silent_cmd.exe myscript.bat
silent_cmd.exe myscript.bat arg1 arg2
silent_cmd.exe --delay 5 myscript.bat
silent_cmd.exe --log out.log myscript.bat
silent_cmd.exe --dir "C:\Work" myscript.bat
silent_cmd.exe --nowait myscript.bat
silent_cmd.exe --delay 5 --dir "C:\Work" --log out.log myscript.bat arg1 arg2
```

終了コード：コマンド自身の終了コード（`--nowait` では起動できた時点で `0`）。起動できない場合や `--log` のファイルを作成できない場合は `1`、コマンドがない場合やオプションが不完全な場合は `2`。

コンソールなしで起動された場合（ショートカット、タスク スケジューラ）、コマンドの標準入力は空になります。`pause` や `set /p` は、見えないコンソールを待ち続けることなく、すぐに戻ります。使われるのは常にシステムの `cmd.exe` で、レジストリの AutoRun コマンドは無効になります（`/D`）。

`.bat` を直接ダブルクリックすると、必ず一瞬コンソールが表示されます。これは `cmd.exe` 自身によるもので、Silent CMD ではありません。完全に見えない起動にするには、ダブルクリックの対象を Silent CMD 自身にしてください。たとえば、スクリプトを引数にした `silent_cmd.exe` へのショートカットです。`.bat` を直接対象にしてはいけません。

## ライセンス

個人的かつ非商用の利用に限り無料です。Copyright (C) 2026 Nyaldee, all rights reserved. 条件の全文は [LICENSE](../LICENSE) をご覧ください。ダウンロードは、このリポジトリの [Releases](https://github.com/Nyaldee/nyaruhodo/releases) からのみ行ってください。

このプロジェクトのコードには、Anthropic の AI である [Claude](https://claude.com) が貢献しています。
