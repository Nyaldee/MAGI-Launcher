# CASPER Shot

*[Read in English](README.md) · [Lire en français](README.fr.md) · [阅读中文版](README.zh-CN.md)*

Windows 用の小さなスクリーンショットツールです。グローバルホットキーが 2 つ、WebP で保存、通知領域にアイコン。ウィンドウはなく、インストールも不要です。

## 機能

- グローバルホットキーで、カーソルのあるモニターをキャプチャします（既定は `PrintScreen`）。
- グローバルホットキーで、ドラッグして範囲を選択するか、ウィンドウをクリックしてそのウィンドウ全体を選択できます。どのモニターでも使えます（既定は `Shift+PrintScreen`）。Esc キーまたは右クリックでキャンセルします。どちらのホットキーも変更できます（下記参照）。
- 通知領域のアイコンを左クリックすると、ホットキーなしで範囲選択モードに入ります。
- キャプチャはすべてクリップボードにコピーされ、`YYYY-MM` サブフォルダーに `.webp` として保存されます。ファイル名はカーソル下のウィンドウ、日付、品質から付けられます（`Discord_06-14-2026_q90.webp`）。既存のファイルは上書きされず、代わりに ` (1)`、` (2)`… が付きます。
- 画面下部にプレビューが少しの間表示されます。左クリックで画像を開き、右クリックでそのフォルダーを開きます。プレビューが次のキャプチャに写り込むことはありません。
- アイコンのメニュー（右クリック）：ホットキーの有効化／無効化、スクリーンショットフォルダーを開く、Settings、Reload、Website（nyaruhodo.com を開く）、終了。

## 設定（`state.json`）

初回起動時に実行ファイルの隣に作成されます。アイコンのメニュー → Settings で開き（`.json` に関連付けられたアプリがなければメモ帳で開きます）、Reload で変更を反映します。ファイルの JSON が不正な場合、CASPER Shot は既定値を使い、修正できるようファイルには手を付けません。

```json
{
  "quality": 90,
  "hotkey_fullscreen": "PrintScreen",
  "hotkey_region": "Shift+PrintScreen",
  "save_path": "%USERPROFILE%\\Pictures\\Screenshots",
  "preview_percent": 40,
  "preview_duration_ms": 2500
}
```

- `quality`：0〜99 は非可逆 WebP、100 は可逆です。
- `hotkey_*`：`Ctrl`／`Alt`／`Shift`／`Win` の任意の組み合わせに、英字、数字、F1〜F12、または `PrintScreen` を加えたもの。
- `save_path`：`%ENV_VAR%` 形式の環境変数を展開します。相対パスは実行ファイルのフォルダーを基準にします。`%USERPROFILE%\Pictures`（Desktop、Documents、Downloads、Music、Videos も同様）は、OneDrive を含め、Windows がフォルダーを移動した先に追従します。
- `preview_percent`：プレビューの大きさ（画面の幅と高さに対する %）。`0` でプレビューを完全に無効にします。
- `preview_duration_ms`：プレビューが閉じるまでの表示時間。

## ホットキーが効かない場合

`RegisterHotKey` では、あるキーの組み合わせをシステム全体で所有できるプロセスは 1 つだけです。実行中の別のアプリが同じ組み合わせを使っている場合（ShareX、Greenshot、設定 → アクセシビリティ → キーボードにある Windows の画面キャプチャのショートカットなど）、CASPER Shot の登録は何も表示せずに失敗し、アイコンのツールチップにはショートカットの代わりに「hotkeys unavailable」と表示されます。そのアプリを終了するか設定を変えるか、`state.json` で別のショートカットを選んで Reload してください。

## 更新

`CASPER Shot` フォルダーから `casper_shot_updater.bat` を実行してください。CASPER Shot を終了し、最新リリースをダウンロードしてその場に展開し、再び起動します。`state.json` とキャプチャはそのまま残ります。

## ライセンス

個人的かつ非商用の利用に限り無料です。Copyright (C) 2026 Nyaldee, all rights reserved. 条件の全文は [LICENSE](../LICENSE) をご覧ください。ダウンロードは、このリポジトリの [Releases](https://github.com/Nyaldee/nyaruhodo/releases) からのみ行ってください。

このプロジェクトのコードには、Anthropic の AI である [Claude](https://claude.com) が貢献しています。
