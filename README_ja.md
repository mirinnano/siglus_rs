# siglus_rs

<img src="./icon/Icon.png" alt="icon" align="left" width="100" style="margin-right: 10px;" />

**siglus_rs** は、**UK2、AVG32、RealLive、SiglusEngine** の非公式な完全Rust再実装プロジェクトであり、主に SiglusEngine に焦点を当てています。

本プロジェクトは非商用であり、研究・調査目的で開発されています。

<br clear="left"/>

## スクリーンショット例

### UK2 Engine
**1993年–1997年 · AyPio**

<table>
  <tr>
    <td align="center" width="33%">
      <img src="./images/uk2_1.png" width="100%"><br>
      <b>ソーサー・キングダム (Sorcer Kingdom)</b><br>
      <sub>タイトル画面</sub>
    </td>
    <td align="center" width="33%">
      <img src="./images/uk2_2.jpeg" width="100%"><br>
      <b>ソーサー・キングダム (Sorcer Kingdom)</b><br>
      <sub>会話シーン</sub>
    </td>
    <td align="center" width="33%">
      <img src="./images/uk2_3.png" width="100%"><br>
      <b>ソーサー・キングダム (Sorcer Kingdom)</b><br>
      <sub>マップ探索</sub>
    </td>
  </tr>
</table>


### AVG32
**1996年–2001年 · VisualArt's**

<table>
  <tr>
    <td align="center" width="33%">
      <img src="./images/avg32_air_title.png" width="100%"><br>
      <b>AIR</b><br>
      <sub>AIR タイトル画面</sub>
    </td>
    <td align="center" width="33%">
      <img src="./images/avg32_air_op.png" width="100%"><br>
      <b>AIR</b><br>
      <sub>AIR オープニング</sub>
    </td>
    <td align="center" width="33%">
      <img src="./images/avg32_kanon.png" width="100%"><br>
      <b>Kanon</b><br>
      <sub>Kanon オープニング</sub>
    </td>
  </tr>
</table>


### RealLive
**2001年–2022年 · VisualArt's**

<table>
  <tr>
    <td align="center" width="33%">
      <img src="./images/rl_lb.jpeg" width="100%"><br>
      <b>リトルバスターズ！</b><br>
      <sub>リトルバスターズ！ スクリーンショット</sub>
    </td>
    <td align="center" width="33%">
      <img src="./images/rl_game.jpeg" width="100%"><br>
      <b>リトルバスターズ！</b><br>
      <sub>リトルバスターズ！ 野球ミニゲーム (bt00.dll 実装)</sub>
    </td>
    <td align="center" width="33%">
      <img src="./images/rl_tomoyo_dt.jpeg" width="100%"><br>
      <b>智代アフター ～It's a Wonderful Life～</b><br>
      <sub>Dungeons & Takafumis ミニゲーム (dt00.dll)</sub>
    </td>
  </tr>
</table>


### SiglusEngine
**2010年–現在 · VisualArt's**

<table>
  <tr>
    <td align="center" width="33%">
      <img src="./images/screenshot.png" width="100%"><br>
      <b>macOS</b><br>
      <sub>macOS上でネイティブ動作するSiglusEngine</sub>
    </td>
    <td align="center" width="33%">
      <img src="./images/ios-screenshot.png" width="100%"><br>
      <b>iOS</b><br>
      <sub>iOS上で動作するSiglusEngine</sub>
    </td>
    <td align="center" width="33%">
      <img src="./images/screenshot-wasm.png" width="100%"><br>
      <b>WebAssembly</b><br>
      <sub>Webブラウザ上で動作するSiglusEngine</sub>
    </td>
  </tr>
</table>

<table>
  <tr>
    <td align="center" width="33%">
      <img src="./images/sg_benchmark.png" width="100%"><br>
      <b>ちはやローリング WE</b><br>
      <sub>Rewriteのパフォーマンス測定用にKeyが公開した公式ベンチマーク</sub>
    </td>
    <td align="center" width="33%">
      <img src="./images/sg_summer.png" width="100%"><br>
      <b>Summer Pockets REFLECTION BLUE</b><br>
      <sub>新ルートとヒロインを追加したSummer Pockets拡張版</sub>
    </td>
    <td align="center" width="33%">
      <img src="./images/sg_anemoi.png" width="100%"><br>
      <b>anemoi</b><br>
      <sub>Keyによる2026年恋愛アドベンチャー作品</sub>
    </td>
  </tr>
</table>

### ゲーム機対応

<table>
  <tr>
    <td align="center" width="33%">
      <img src="./images/psvita.png" width="100%"><br>
      <b>PS Vita</b><br>
      <sub>PS Vita版 Summer Pockets REFLECTION BLUE</sub>
    </td>
    <td align="center" width="33%">
      <img src="./images/switch.png" width="100%"><br>
      <b>Nintendo Switch</b><br>
      <sub>Nintendo Switch版 anemoi</sub>
    </td>
  </tr>
</table>

* siglus_rs は Windows、Linux、macOS、iOS、Android、WebAssembly を含む幅広いプラットフォームで動作します。

| プラットフォーム | 対応ターゲット |
|---|---|
| Linux | x86_64, aarch64 |
| FreeBSD | x86_64 |
| Windows | x86_64, ARM64 |
| macOS | aarch64 app, x86_64 app, universal DMG app bundle |
| iOS | arm64 デバイス, arm64 シミュレータ, x86_64 シミュレータ |
| Android | arm64-v8a, x86_64 |
| WebAssembly | wasm32-unknown-unknown |
| PS Vita | armv7 |
| Nintendo Switch | aarch64 |

アプリランチャー（macOSバンドル、iOS、Android、WebAssembly）は、1つまたは複数のゲームを一度にインポートし、エンジン（SiglusEngine, RealLive, AVG32, UK2）を自動検出してカバーアートやアプリアイコンを表示します。また、Siglus以外のゲームではテキストエンコーディングの選択も可能です。詳細は [crates/game_launcher](crates/game_launcher/README.md) を参照してください。

## ビルド済みバイナリ
* プレビューリリースは [GitHub Releases](https://github.com/xmoezzz/siglus_rs/releases) をご覧ください。

## ドキュメント
* APIドキュメント: [docs](https://xmoezzz.github.io/siglus_rs/)
* [PS Vita 移植版](platform/vita/README.md) — 実験的 SiglusEngine プレイヤー（リリースページの `siglus-psvita.vpk`）。本体およびゲームの導入方法は該当ドキュメントに記載されています。[ロードマップ](platform/vita/ROADMAP.md)。

## 実行方法

```bash
cargo run --release -p siglus_scene_vm --bin siglus_engine -- --project-dir ~/Documents/siglus_rs-main/testcase
```

デスクトップのゲームウィンドウを閉じる際、ゲーム独自の終了確認ダイアログが優先されます。
エンジンは共有スクリプトコマンド、設定されたキャンセルメニューシーン、アクティブなタイトルメニューのローカルボタンアクションの中から、引数のない一意の終了アクションを探します。タイトルアクションはタイトルボタンが準備完了した後にのみ利用可能です。EXCALLメニュー保存が必要なスクリプト、サポートされていないまたは曖昧なスクリプト、およびすでにアクティブなシステムメニューでは、代わりに組み込みの終了確認を使用します。
`Gameexe.ini` のオプション項目 `#CLOSE_SCENE = "scene_name", label` でこの動作をオーバーライドできます。指定されたシーンで終了確認を処理し、終了を受け入れた場合は `syscom.end_game` を呼び出す必要があります。戻るとゲームが再開されます。

デスクトップウィンドウのアイコンは、ゲームディレクトリ内の `icon.png`、`icon.ico`、またはファイル名順で最初に読み込める `.ico` ファイルが使用され、見つからない場合は Siglus アイコンにフォールバックします。
エンジンは winit `0.31.0-beta.3` を使用し、対応する Wayland コンポジタでは `xdg_toplevel_icon_v1` 経由でウィンドウアイコンを直接送信します。デスクトップエントリは生成されません。各ゲームは個別の Wayland アプリケーション ID を保持します。

## コミュニティ
開発やディスカッションに参加したい場合は、以下の Discord サーバーに参加できます：
* Discord: [https://discord.gg/g4rXucPZz3](https://discord.gg/g4rXucPZz3)
* （作者より）英語、中国語、日本語、およびごく限定的なフランス語に対応しています。

## 設定ファイル `key.toml`
`key.toml` ファイルは、siglus_rs の各種設定オプションを指定するために使用されます。このファイルはゲームプロジェクトのルートディレクトリに配置してください。

### リソース復号キー
* この要素の設定キーは `key` です。セカンダリキーを表す 16 バイト（128 ビット）の配列です。
* SiglusEngine のゲームは、保護されたリソースを復号するためにセカンダリキーを必要とします。

siglus_rs は自動的にセカンダリキーを総当たり（ブルートフォース）で解析できますが、手動でキーを指定したい場合は、ゲームのルートディレクトリに `key.toml` ファイルを作成できます。

キーを取得する実用的な方法がいくつかあります：

1. 静的抽出：ゲーム実行ファイルが暗号化や難読化されていない場合（推奨）。基本的な手法は以下のリポジトリを参照してください：
   https://github.com/xmoezzz/siglus_static_key_tool

2. 動的抽出：基本的な手法は以下の古いリポジトリを参照してください：
   https://github.com/xmoezzz/SiglusExtract

3. 一部の抽出ツールが管理している既知キーのデータベース。

以下の状況でブルートフォースが試行されます：
1. `key.toml` ファイルにキーが指定されていない場合、siglus_rs はキーのブルートフォースを試みます。
2. ユーザーが `key.toml` に誤ったキーを指定した場合、siglus_rs は正しいキーで上書きを試みます。
3. `key.toml` ファイルの保存や上書きに失敗した場合でも、エンジンはそのまま実行されます。

体験版ゲームではセカンダリキーが不要な場合があり、その場合は `key.toml` でオールゼロのキーを指定できます。`key.toml` の例：

```toml
key = [
  0x00, 0x00, 0x00, 0x00,
  0x00, 0x00, 0x00, 0x00,
  0x00, 0x00, 0x00, 0x00,
  0x00, 0x00, 0x00, 0x00,
]
```

### 文字列暗号化
* この要素の設定キーは `override_string_encryption` です。値は `xor`、`none`、`mdl` のいずれかを指定できます。デフォルト値は `xor` です。
* ごく初期の SiglusEngine バージョンでは、文字列暗号化は使用されていませんでした。
* しかし、その後のバージョンでは（ほとんどの場合）、デフォルトで文字列暗号化が有効になっています。

各値の説明：
* `xor`: 文字列暗号化を有効にします。`key.toml` に `override_string_encryption` が指定されていない場合でも、これがデフォルト値になります。
* `none`: 文字列暗号化を無効にします。ゲームが文字列暗号化を使用していないと確信できる場合に使用します。
* `mdl`: MDL アプローチ（論文参照: [https://arxiv.org/abs/cs/0312044](https://arxiv.org/abs/cs/0312044)）を使用して、文字列暗号化方式を自動判別します。多少のパフォーマンスオーバーヘッドが発生しますが、作者の見解としては軽微です。

## ライセンス
本プロジェクトは MPL-2.0 ライセンスの下でライセンスされています。詳細は [LICENSE](./LICENSE-MPL-2.0) を参照してください。
