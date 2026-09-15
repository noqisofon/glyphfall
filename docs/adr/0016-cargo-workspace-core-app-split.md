# ADR-0016: Cargoワークスペース化によるロジックコア（`glyphfall-core`）とBevy皮（`glyphfall`/`app`）の分離

- Status: Accepted
- Date: 2026-09-15

## Background

将来的にSSH経由でも遊べるようにする（クライアント/サーバー分離、テキスト端末での
プレイ）ための下地として、ゲームロジックがBevyに依存しない形で単独ビルド・単独
テストできる状態を作る必要があった。今回のスコープはあくまで「コアを分離する」
ところまでであり、SSHサーバー・クライアントの実装や通信プロトコルの設計は含まない。

事前の見立てでは、`battle::BattleState` / `party::PartyState` / `town::TownState`
などのロジック本体は実質すでにプレーンなRust構造体＋メソッドとして書かれており、
Bevy依存は`#[derive(Component)]` `#[derive(Resource)]`というderiveマクロのみ、
`ui::view::format_*`系はすでに「状態を受け取って文字列を返す純粋関数」になっている
はず、という想定だった。

## Circumstances

実際にコードを読むと、上記の想定は一部食い違っていた。

1. **`Monster` / `PartyMember` / `Influence`**: `#[derive(Component)]`が付与
   されていたが、実際にECSエンティティとして`.spawn()`されている箇所は皆無で、
   常に`Vec<Monster>` / `Vec<PartyMember>`としてプレーンなデータとして扱われて
   いた。derive削除のみで安全に切り離せた。
2. **`BattleState::flash_timer`**: 型が`bevy::time::Timer`そのものであり、
   `#[derive(Resource)]`という表面上のderiveだけでなく、フィールドの型自体が
   Bevy依存だった。モンスターがダメージを受けた際の明滅演出用のカウントダウンで、
   `app`側（`ui/animation.rs`の`monster_flash_tick`）が`Res<Time>`から得た
   `Duration`で毎フレーム`tick()`している。
3. **`town::TownState`**: マップ・視界・プレイヤー位置などの純粋な探索状態に加えて、
   `Handle<Image>` / `Assets<Image>` / `RenderAssetUsages` / `TextureFormat`
   など、ドット絵街マップをBevyの`Image`アセットとして生成・更新するロジックが
   同じ構造体に同居していた。これは`town::pixel_art`（RGBAピクセルバッファ生成、
   Bevy非依存）の出力をBevyのテクスチャに橋渡しする、明確にUI/描画側の関心事。
4. **Bevyの`Resource`トレイトはBevy側の外部トレイトであり、`glyphfall-core`と
   `glyphfall`（app）が別クレートになる以上、孤児則（orphan rule）により
   `app`側で`impl Resource for glyphfall_core::battle::BattleState`のような
   実装は書けない。

## Rationale

1. **`Monster` / `PartyMember` / `Influence`等の未使用derive**: 実害のない
   死んだコードなので、単純に削除する。ラッパーは不要。
2. **`BattleState::flash_timer`**: 「演出用カウントダウン」というロジック自体は
   ゲームルールの一部（連続ダメージ表現）というよりUI関心事に近いが、
   `BattleState`という1つの構造体を`core`/`app`に分割するのは複雑さに見合わない。
   代わりに、`bevy::time::Timer`が内部で使っている`std::time::Duration`ベースの
   最小限のカウントダウン実装（`core::timer::SimpleTimer`）を用意し、
   `BattleState`はそのまま`core`に置く。`app`側は`Res<Time>::delta()`（これ自体は
   `std::time::Duration`）を渡すだけなので、呼び出し側の変更はごくわずかで済む。
3. **`TownState`**: こちらは`Image`アセット管理という明確にBevy依存な責務を
   含むため、SimpleTimerのような小手先の型置換では済まない。
   `core::town::TownState`にはマップ・視界・プレイヤー位置・向き・追従履歴・
   松明状態・「再描画が必要か」を示す`dirty`フラグのみを残し、テクスチャの
   生成（`Image::new`等）と`texture_handle: Handle<Image>`の保持は
   `app::town::TownStateRes`に切り出す。`dirty`フラグ自体はBevy型を含まない
   単なる`bool`なので、実装の単純さを優先してコア側に残す。
4. **Bevy `Resource`化の孤児則対応**: コアの各状態型（`BattleState` /
   `PartyState` / `TownState` / `PlayerInventory` / `PlayerResource` /
   `ReserveRoster` / `SuddenEventRegistry` / `SuddenEventHistory`）は、
   `app`側で`XxxRes(pub CoreType)`というニュータイプに包み、そちらへ
   `#[derive(Resource)]`を付与する。素朴な`.0`アクセスへの機械的置換は
   フィールド参照が非常に多いため、代わりに`Deref` / `DerefMut`を実装し、
   `Res<XxxRes>` / `ResMut<XxxRes>`のフィールドアクセス・メソッド呼び出しが
   これまでどおり動くようにする（Rustの参照外し型強制は多段階のDerefチェーンを
   自動的にたどるため、`&Res<XxxRes>` → `&XxxRes` → `&CoreType`のような
   二段階の型強制も問題なく機能する）。これにより`app`側の書き換え量を
   最小化しつつ、コア側は完全にBevyから独立させられる。

## Decision

1. ワークスペースルートの`Cargo.toml`を`[workspace] members = ["core", "app"]`
   のみの定義に置き換え、`core/`（`glyphfall-core`、依存は`rand`のみ）と
   `app/`（`glyphfall`、依存は`glyphfall-core` + `bevy` + `rand`）の2クレートに
   分割する。
2. `core`には次を移す: `battle`（`BattleState`等。`flash_timer`は
   `core::timer::SimpleTimer`に置き換え）、`party`（`action` / `diagnosis` /
   `influence` / `inventory`、Component/Resourceのderiveは削除）、
   `town`（`dialogue` / `fov` / `interact` / `map` / `movement`、および
   テクスチャ管理を除いた`TownState`）、`event`（`SuddenEventRegistry` /
   `SuddenEventHistory`、Resourceのderiveは削除）。
3. `app`には次を残す: `battle/input.rs`・`town/input.rs`（Bevy入力ハンドラ）、
   `town/pixel_art.rs`（RGBAピクセルバッファ生成。Bevy非依存だが描画データ生成
   としてUI側の関心事）、`ui/`一式、`main.rs`。加えて、コアの状態型をBevyの
   `Resource`として載せるための`XxxRes`ニュータイプ（`Deref`/`DerefMut`付き）を
   `app::battle` / `app::party` / `app::town` / `app::event`に追加する。
   `TownStateRes`はコアの`TownState`に加えて`texture_handle: Handle<Image>`を
   保持し、`new()` / `update_texture()`でBevyの`Image`アセット生成・更新を担う。
4. 受け入れ条件として、`cargo build -p glyphfall-core`がBevyを一切ダウンロード・
   コンパイルせずに成功すること、`cargo test --workspace`で既存のユニット
   テスト（`event`モジュールの4本を含む）がすべて`core`側で通ることを確認した。
   `ui::view`の関数群をテキストプロトコルとして再設計する話や、SSHサーバー・
   クライアントの実装そのものは、引き続き将来の課題として持ち越す。
