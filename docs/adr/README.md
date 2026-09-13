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
| [ADR-0007](0007-pure-glyph-ascii-graphics.md) | グラフィック方針としてピュア・グリフ／ASCIIアート表現の採用 | Accepted | 2026-09-13 |

 
## フォーマットについて
[ADR-0000](0000-adr-template.md) に基づき、暗黙知を持たない未来の自分やLLMに向けて判断基準を正確に伝達できるよう、以下の4段構成を採用しています：

1. **Background**: そもそもなぜこの決定が必要になったか。前提となる状況・制約。
2. **Circumstances**: その時点で具体的に何が起きていたか。選択肢やトリガー。
3. **Rationale**: Circumstancesからどう判断したか。判断基準・優先順位。
4. **Decision**: 結論（Rationaleから導出される決定）。

