# dsh-desktop

DeepSeek Harness (DSH) の Web GUI をデスクトップアプリとしてラップする、薄い Tauri v2 ラッパーです。

## 動作

1. 起動時に `dsh web` を子プロセスとして起動し、DSH Web GUI (`http://127.0.0.1:3080`、環境変数 `DSH_WEB_URL` で上書き可) の準備ができるまでスプラッシュ画面を表示
2. サーバーが応答したら、ウィンドウを Web GUI に遷移
3. アプリ終了時に `dsh web` をプロセスごと終了 (Unix ではプロセスグループに SIGTERM)

`dsh web` がすでに起動済みの場合は、そのサーバーに接続します (起動済みプロセスは残ります)。

## 必要環境

- Node.js + npm
- Rust (rustup でインストール。`source ~/.cargo/env` が必要なシェルあり)
- Tauri v2 の Linux システム依存 (`libwebkit2gtk-4.1-dev` 等)
- DSH CLI — `dsh web` が PATH から実行できること (`~/.local/bin` が PATH に含まれる必要あり)

## 開発

```bash
npm install
npm run tauri dev    # デバッグビルド。`dsh web` のログはターミナルに出力される
```

## ビルド

```bash
npm run tauri build  # 成果物は src-tauri/target/release/bundle/
```

## 構成

- `src/` — スプラッシュ画面 (起動直後にだけ表示されるローカル HTML。サーバー起動待ちの間の表示)
- `src-tauri/src/lib.rs` — `dsh web` の起動・起動監視・終了処理とウィンドウ遷移をすべて担当

## ライセンス

MIT License — 詳しくは [LICENSE](LICENSE) を参照。