# Glyphfall プロトタイプ

社内コードネーム: Glyphfall（罫線文字＝グリフ + Daggerfall由来の"fall"）

罫線文字ウィンドウ枠 + 等幅フォントのメッセージボックス最小構成。

## セットアップ

1. 等幅フォントを用意する
   - `assets/fonts/FiraMono-Regular.ttf` に等幅フォントを配置してください
   - お好みで `Cica`, `HackGen`, `JetBrains Mono` などの日本語対応等幅フォントに差し替え可
   - `src/main.rs` 内 `FONT_PATH` を変更すればファイル名も変えられます

2. 実行

```bash
cargo run
```

初回は Bevy のコンパイルに数分かかります（`dynamic_linking` feature で2回目以降は高速化されます）。

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



## 次にやりそうなこと（メモ）

- 罫線を二重線・太線に切り替えられるようにする（`box_chars` を enum 化）
- コマンドウィンドウ（たたかう/じゅもん/どうぐ/にげる）を同じ枠組み関数で追加
  - 選択カーソルは `▶` などの罫線系記号で表現すると一貫性が出そう
- CRT風スキャンライン・グローのポストプロセス（`bevy::core_pipeline` のカスタムポストプロセスパスで別途追加）
- メッセージ送り中にキー入力で全文即表示にスキップする処理
- 会話メモにある「話題ウィンドウ／行動ウィンドウ」の三分割レイアウトへの拡張
  - 今回の `spawn_message_window` はレイアウトのプリミティブとして
    そのまま複数配置に転用できる設計にしてある

## アーキテクチャ決定記録 (ADR)

ゲームデザイン思想、システム仕様、技術選定の決定事項は [docs/adr/](docs/adr/README.md) に記録されています。
- [ADR-0000: ADRのフォーマットを4段構成にする](docs/adr/0000-adr-template.md)
- [ADR-0001: Daggerfall × DQ ハイブリッドゲームデザインの採用](docs/adr/0001-daggerfall-dq-hybrid-concept.md)
- [ADR-0002: 三分割会話UIと手動キーワード記憶（「覚える」メカニクス）](docs/adr/0002-tri-split-conversation-and-memorization.md)
- [ADR-0003: 隠しパラメータ「影響度」によるパーティ自律行動制御](docs/adr/0003-party-influence-and-autonomous-behavior.md)
- [ADR-0004: 旅シミュレーション型ファストトラベルと創発イベント](docs/adr/0004-travel-simulation-and-emergent-events.md)
- [ADR-0005: 主人公の非英雄性（パンピー）と勇者キャンペーンの分離](docs/adr/0005-commoner-protagonist-and-hero-campaign-split.md)
- [ADR-0006: Bevy ECS と罫線グリッドによるターミナルUIアーキテクチャ](docs/adr/0006-glyph-grid-terminal-ui-architecture.md)
- [ADR-0007: グラフィック方針としてピュア・グリフ／ASCIIアート表現の採用](docs/adr/0007-pure-glyph-ascii-graphics.md)
- [ADR-0008: ハイブリッド移動視点システム（視界制限付き見下ろし × 一人称疑似3D）の採用](docs/adr/0008-hybrid-exploration-perspective.md)



