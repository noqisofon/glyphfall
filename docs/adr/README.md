# Glyphfall アーキテクチャ決定記録 (ADR)
 
プロジェクト「Glyphfall」における主要なゲームデザイン、システム設計、技術スタックの選定およびアーキテクチャ上の意思決定を記録します。
 
## ADR 一覧
 
| 番号 | タイトル | ステータス | 決定日 |
| :--- | :--- | :--- | :--- |
| [ADR-0000](0000-adr-template.md) | ADRのフォーマットを4段構成にする | Accepted | 2026-09-10 |
| [ADR-0001](0001-daggerfall-dq-hybrid-concept.md) | Daggerfall × DQ ハイブリッドゲームデザインの採用 | Accepted | 2026-09-13 |
| [ADR-0002](0002-tri-split-conversation-and-memorization.md) | 三分割会話UIと手動キーワード記憶（「覚える」メカニクス） | Accepted | 2026-09-13 |
| [ADR-0003](0003-party-influence-and-autonomous-behavior.md) | 隠しパラメータ「影響度」によるパーティ自律行動制御 | Accepted | 2026-09-13 |
| [ADR-0004](0004-travel-simulation-and-emergent-events.md) | 旅シミュレーション型ファストトラベルと創発イベント | Accepted | 2026-09-13 |
| [ADR-0005](0005-commoner-protagonist-and-hero-campaign-split.md) | 主人公の非英雄性（パンピー）と勇者キャンペーンの分離 | Accepted | 2026-09-13 |
| [ADR-0006](0006-glyph-grid-terminal-ui-architecture.md) | Bevy ECS と罫線グリッドによるターミナルUIアーキテクチャ | Accepted | 2026-09-13 |
| [ADR-0007](0007-pure-glyph-ascii-graphics.md) | グラフィック方針としてピュア・グリフ／ASCIIアート表現の採用 | Superseded | 2026-09-13 |
| [ADR-0008](0008-hybrid-exploration-perspective.md) | ハイブリッド移動視点システム（視界制限付き見下ろし × 一人称疑似3D）の採用 | Superseded | 2026-09-13 |
| [ADR-0009](0009-retro-2d-sprite-graphics.md) | レトロ2Dドット絵／スプライト・ハイブリッドの採用 | Accepted | 2026-09-14 |
| [ADR-0010](0010-unified-top-down-exploration.md) | 統一見下ろし視点探索システムとダンジョン2Dドット絵化の採用 | Accepted | 2026-09-14 |
| [ADR-0011](0011-command-driven-interaction.md) | コマンド駆動インタラクト（Zキー→コマンド選択→方向選択）の採用 | Accepted | 2026-09-15 |
| [ADR-0012](0012-oboeru-underline-phrase-selection.md) | メッセージ内下線語句選択方式による「おぼえる」UXの刷新 | Accepted | 2026-09-15 |
| [ADR-0013](0013-sudden-event-trigger-system.md) | 突発イベント発生システム（トリガー／抽選エンジン）の設計 | Accepted | 2026-09-15 |
| [ADR-0014](0014-input-decomposition-and-reactive-ui.md) | 入力処理のモード別分割とチェンジディテクションによるUI更新アーキテクチャ | Accepted | 2026-09-15 |
| [ADR-0015](0015-turn-based-battle-and-module-decomposition.md) | 仲間指示型ターン制戦闘ループと「あなた」実体化・ファイル責務再編 | Accepted | 2026-09-16 |
| [ADR-0016](0016-cargo-workspace-core-app-split.md) | Cargoワークスペース化によるロジックコア（`glyphfall-core`）とBevy皮（`glyphfall`/`app`）の分離 | Accepted | 2026-09-15 |

 
## フォーマットについて
[ADR-0000](0000-adr-template.md) に基づき、暗黙知を持たない未来の自分やLLMに向けて判断基準を正確に伝達できるよう、以下の4段構成を採用しています：

1. **Background**: そもそもなぜこの決定が必要になったか。前提となる状況・制約。
2. **Circumstances**: その時点で具体的に何が起きていたか。選択肢やトリガー。
3. **Rationale**: Circumstancesからどう判断したか。判断基準・優先順位。
4. **Decision**: 結論（Rationaleから導出される決定）。

