# Glyphfall プロトタイプ

社内コードネーム: Glyphfall（罫線文字＝グリフ + Daggerfall由来の"fall"）

罫線文字ウィンドウ枠 + 等幅フォントのメッセージボックス最小構成。

## セットアップ

1. 等幅フォント
   - 標準で **Bizin Gothic**（`assets/fonts/BizinGothic-Regular.ttf`）を適用しています。
   - **Mint Mono**（`assets/fonts/MintMono-Regular.ttf`）も同梱しています。
   - `app/src/ui/mod.rs` 内 `FONT_PATH` で即座に切り替え可能です。
   - （その他、HackGen や BIZ UDゴシック も同梱済み）

2. 実行

```bash
cargo run -p glyphfall
```

初回は Bevy のコンパイルに数分かかります（`dynamic_linking` feature で2回目以降は高速化されます）。

## ワークスペース構成（ADR-0016）

Cargoワークスペースとして `core/`（`glyphfall-core`: Bevy非依存のゲームロジック）と
`app/`（`glyphfall`: Bevy製の実行バイナリ）に分割しています。

```bash
cargo build -p glyphfall-core  # ロジックのみ。Bevyは一切ビルドされない
cargo test -p glyphfall-core   # ロジックのユニットテスト
cargo run -p glyphfall         # ゲームを起動
```

## 今の実装が持っているもの

- 罫線文字（`┌─┐│└─┘`）でグリッド計算してウィンドウ枠を自動生成
  - `spawn_message_window(parent, font, inner_cols, inner_rows, message)` で
    サイズと中身を指定するだけで枠が組み上がる
- 緑モノクロのターミナル配色（`palette` モジュールで一括管理）
- DQ的な1文字ずつのメッセージ送り演出（`TypewriterMessage` + `typewriter_tick`、Spaceで即時スキップ）
- **影響度＆パーティ自律行動システム（ADR-0003）**
  - 自然上限89の隠しパラメータ `Influence`、外的魔術介入による100超の精神異常
  - 指示（たたかう／みをまもる／すてみ）に対する対抗判定と不服従・自律行動（遊び人のサボり、ヤンデレの過保護等）
  - 「魔術の知識」＋「目星」による精神異常の見抜き／ヤンデレ（素の執着）の判別
  - 上部ステータス枠、[Tab]仲間切替、[F1]デバッグ用隠し数値可視化トグル
- **ピュア・グリフ／ASCIIアート戦闘ビュー（ADR-0007）**
  - 中央ウィンドウにモンスターのASCIIグリフアート＋HPバー `[████████      ]` を常時表示
  - スライムグリフ、がいこつせんし、キラーベア、山賊のとうぞくを文字記号で表現
  - 被弾時にモンスターが赤く点滅するフラッシュ演出、モンスター撃破と次世代リポップ
- **レトロ2Dドット絵／スプライト・ハイブリッド（ADR-0009）**
  - 中央ウィンドウに 16×16 ピクセルのレトロドット絵（736×144pxテクスチャ）で街やダンジョンを鮮やかに描画
  - プロポーショナルフォントや文字幅問題から脱却し、1ピクセルのズレもなく整列したゲーム画面を実現
  - 主人公（旅人・青マント）、戦士ガルツ（赤兜）、遊び人ロロ（道化帽）、魔法使いミレイ（とんがり帽）、騎士アルヴィン（白銀鎧）をドット絵で描き下ろし
  - 松明（`[L]` キー）の光量に応じたシャドウマスクによる滑らかな視界制限（FOV）演出
- **統一見下ろし探索システム＆地下迷宮B1F（ADR-0010）**
  - 王都アルカンと「封魔の地下迷宮 B1F」を、同じ見下ろし2Dドット絵で仲間を引き連れて探索
  - ダンジョン専用ドット絵（苔むした岩壁、ひび割れ石床、鉄格子扉、宝箱、昇降階段、魔物シンボル）
  - 王都アルカンの階段（`>`）から迷宮へ潜り、迷宮の階段（`<`）から街へ生還するフロア移動サイクル
  - 宝箱の開封（フォリン獲得）や魔物シンボル接触による戦闘突入
  - 将来的なプロシージャル迷宮生成（自動生成ダンジョン）への拡張を前提としたマップ設計


- **三分割会話UI＆手動キーワード記憶（ADR-0002）**
  - 下部ウィンドウを横三分割（左：話題/品物、中：メッセージ、右：行動）に拡張し、幾何的グリッド（48列 = 15列 + 22列 + 11列）で中央画面と完全一致
  - 会話相手（王都衛兵、酒場の呑兵衛、裏通りの怪しい男、街の女性、案内看板）に接触して会話開始
  - 「覚える（★）」アクションにより、相手の台詞に含まれる新キーワード（例：「封印の祭壇」「光のオーブ」「銀の鍵」等）を手帳にストック
  - 手帳にストックした話題を別の住人に振ることで、さらなる情報を引き出す聞き込み推理チェーン
- **お店・宿屋・酒場システム**
  - **道具屋（Shop）**: 所持金（フォリン）を消費してやくそう・特やくそう・松明・どくけし草を購入しインベントリに追加
  - **宿屋（Inn）**: 一泊50Gで仲間全員のHPとMPを全回復
  - **宝箱（Chest）**: ダンジョンや街の宝箱からフォリン（120G）や特やくそうを入手

## 次にやりそうなこと（メモ）

- ダンジョンのプロシージャル自動生成（部屋と通路のランダム接続。深層フロア展開はADR-0021でB1F〜B6Fの6階層まで実装済み）
- 旅シミュレーション型ファストトラベルと創発イベント（ADR-0004）
- 罫線を二重線・太線に切り替えられるようにする（`box_chars` を enum 化）
- CRT風スキャンライン・グローのポストプロセス（`bevy::core_pipeline` のカスタムポストプロセスパスで別途追加）

## アーキテクチャ決定記録 (ADR)

ゲームデザイン思想、システム仕様、技術選定の決定事項は [docs/adr/](docs/adr/README.md) に記録されています。
- [ADR-0000: ADRのフォーマットを4段構成にする](docs/adr/0000-adr-template.md)
- [ADR-0001: Daggerfall × DQ ハイブリッドゲームデザインの採用](docs/adr/0001-daggerfall-dq-hybrid-concept.md)
- [ADR-0002: 三分割会話UIと手動キーワード記憶（「覚える」メカニクス）](docs/adr/0002-tri-split-conversation-and-memorization.md)
- [ADR-0003: 隠しパラメータ「影響度」によるパーティ自律行動制御](docs/adr/0003-party-influence-and-autonomous-behavior.md)
- [ADR-0004: 旅シミュレーション型ファストトラベルと創発イベント](docs/adr/0004-travel-simulation-and-emergent-events.md)
- [ADR-0005: 主人公の非英雄性（パンピー）と勇者キャンペーンの分離](docs/adr/0005-commoner-protagonist-and-hero-campaign-split.md)
- [ADR-0006: Bevy ECS と罫線グリッドによるターミナルUIアーキテクチャ](docs/adr/0006-glyph-grid-terminal-ui-architecture.md)
- [ADR-0007: グラフィック方針としてピュア・グリフ／ASCIIアート表現の採用 (Superseded)](docs/adr/0007-pure-glyph-ascii-graphics.md)
- [ADR-0008: ハイブリッド移動視点システム（視界制限付き見下ろし × 一人称疑似3D）の採用 (Superseded)](docs/adr/0008-hybrid-exploration-perspective.md)
- [ADR-0009: レトロ2Dドット絵／スプライト・ハイブリッドの採用](docs/adr/0009-retro-2d-sprite-graphics.md)
- [ADR-0010: 統一見下ろし視点探索システムとダンジョン2Dドット絵化の採用](docs/adr/0010-unified-top-down-exploration.md)
- [ADR-0011: コマンド駆動インタラクト（Zキー→コマンド選択→方向選択）の採用](docs/adr/0011-command-driven-interaction.md)
- [ADR-0012: メッセージ内下線語句選択方式による「おぼえる」UXの刷新](docs/adr/0012-oboeru-underline-phrase-selection.md)
- [ADR-0013: 突発イベント発生システム（トリガー／抽選エンジン）の設計](docs/adr/0013-sudden-event-trigger-system.md)
- [ADR-0014: 入力処理のモード別分割とチェンジディテクションによるUI更新アーキテクチャ](docs/adr/0014-input-decomposition-and-reactive-ui.md)
- [ADR-0015: 仲間指示型ターン制戦闘ループと「あなた」実体化・ファイル責務再編](docs/adr/0015-turn-based-battle-and-module-decomposition.md)
- [ADR-0016: Cargoワークスペース化によるロジックコア（`glyphfall-core`）とBevy皮（`glyphfall`/`app`）の分離](docs/adr/0016-cargo-workspace-core-app-split.md)
- [ADR-0017: セルオートマトン法による洞窟型ダンジョンのプロシージャル生成](docs/adr/0017-cave-procedural-dungeon-generation.md)
- [ADR-0018: 確率的な物語テンプレート部屋によるCave生成のハイブリッド化](docs/adr/0018-hybrid-narrative-template-rooms.md)
- [ADR-0019: ゲーム内通貨名を「フォリン」に変更](docs/adr/0019-forin-currency-rename.md)
- [ADR-0020: 話題ウィンドウの語彙拡張とMOD向けLuaスクリプト対応](docs/adr/0020-knowledge-system-and-lua-mod-support.md)
- [ADR-0021: 封魔の地下迷宮の複数階層化（B1F〜B6F）](docs/adr/0021-dungeon-multi-floor-descent.md)
- [ADR-0022: 素性（出自）システムによるゲーム開始パターンの統一 (Proposed)](docs/adr/0022-origin-based-game-start.md)



