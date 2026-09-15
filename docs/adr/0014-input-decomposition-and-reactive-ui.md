# ADR-0014: 入力処理のモード別分割とチェンジディテクションによるUI更新アーキテクチャ

- Status: Accepted
- Date: 2026-09-15

## Background

初期プロトタイプ開発では、等幅罫線グリッド、ドット絵テクスチャ、ASCIIモンスターアート、三分割UIの表示とキー入力の疎通確認を最優先したため、`src/main.rs` 内の単一システム `handle_input` に全画面・全操作のロジックを集約していた。

しかし、機能拡張（街探索、コマンド駆動インタラクト、三分割会話、キーワード記憶、道具屋、宿屋、戦闘）を重ねた結果、`handle_input` は 620 行・引数 16 個に達する巨大な「神システム（God System）」となった。

## Circumstances

この集約構造には、次の具体的な破綻と拡張限界が生じていた：

1. **ECS クエリの破綻（過剰な `Without<...>`）**:
   同一フレーム内で複数のテキストノード（ステータスヘッダー、三分割左・右、モンスター表示、メッセージ本文）を書き換えるため、Bevy の Mutable Aliasing エラーを回避する目的で 1 クエリあたり 4〜5 個もの `Without<...>` を付与せざるを得ず、コードの可読性が著しく低下していた（Clippy `type_complexity` 警告 4 件）。
2. **手動ブールフラグによる UI 同期ミスリスク**:
   `update_header = true` や `update_tri_windows = true` などのローカル手動フラグで画面更新を制御していたため、入力分岐の追加時にフラグを立て忘れると UI の再描画漏れが発生する構造的欠陥を抱えていた。
3. **今後の機能拡張への障害**:
   ADR-0013（突発イベント発生システム）や将来のダンジョン自動生成・追加ミニゲームなどの新モード・新イベントを組み込む際、これ以上 `handle_input` を拡張するのは保守限界を迎えていた。

## Rationale

Bevy ECS 本来の強みである「関心の分離」「チェンジディテクション（Change Detection）」「イベント駆動」を活用し、入力と描画更新を疎結合にする：

1. **メッセージ送信のイベント化**:
   `ShowMessage(pub String)` イベントを導入し、入力システムがテキスト描画コンポーネント（`MessageTextNode`）を直接 mutable クエリする依存を排除する。
2. **UI更新のリアクティブ（自己更新）化**:
   UI ノードごとに独立したシステムを設け、監視対象のリソース（`PartyState`, `AppMode`, `TownState`, `PlayerInventory`, `ActiveDialogue`, `CommandMenuState`）の `is_changed()` を検知して自動描画する。
   これにより、手動フラグ管理を撤廃し、UI ごとの独立クエリになるため過剰な `Without<...>` も全廃される。
3. **モード別入力システムの分割**:
   `AppMode` に応じた run condition（`run_if(in_mode(...))`）を用いて、モードごとに独立した小システム（街・会話・ショップ・宿屋・戦闘・共通）に分割する。各システムの引数は 4〜7 個に収まり、関心のあるリソースのみを注入する。
4. **パイプラインの順序保証**:
   Update スケジュール内で `(タイマー更新, 入力処理, メッセージ/テクスチャ反映, UI再描画).chain()` の順序で実行し、1フレーム内で入力からUI更新までが決定論的・同期的に反映されることを保証する。

## Decision

以下の通り入力システムと UI 更新システムを分離・再構築する：

1. **イベント導入**: `ShowMessage` イベントの登録と、それを受信してタイプライターを初期化する `update_message_window` システムの配置。
2. **モード別入力システムへの分割**:
   - `handle_common_input`: Space（スキップ）、F1（デバッグ）、Tab（注目仲間切替）、B（戦闘/街切替）
   - `handle_town_input`: L（松明）、WASD/矢印（移動）、Z（コマンド開始）
   - `handle_interact_input`: コマンド選択・方向選択（ADR-0011）
   - `handle_dialogue_input`: 話題振り・おぼえる（ADR-0012）
   - `handle_shop_input`: アイテム選択・購入・退店
   - `handle_inn_input`: 宿泊・退店
   - `handle_battle_input`: たたかう・みをまもる・すてみ・観察・敵切替
3. **UI 更新システムの独立**:
   - `update_status_header_system`
   - `update_tri_split_windows_system`
   - `update_battle_monster_display_system`
   - `update_center_window_visibility_system`
   - `update_town_texture_system`
4. **スケジュールのパイプライン化**:
   入力系から UI 反映系までを `.chain()` で結合し、Clippy の `too_many_arguments` および `type_complexity` 警告を完全に解消する。
