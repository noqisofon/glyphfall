use super::{
    palette, BattleMonsterTextNode, MessageHighlightNode, MessageTextNode, MessageUnderlineNode,
    TypewriterMessage,
};
use crate::{
    battle::BattleStateRes,
    town::{DialogueStage, LearnableSpan},
    ActiveDialogue, AppMode, ShowMessage,
};
use bevy::prelude::*;

pub fn typewriter_tick(
    time: Res<Time>,
    mut query: Query<(&mut TypewriterMessage, &mut Text), With<MessageTextNode>>,
) {
    for (mut msg, mut text) in &mut query {
        if msg.shown_chars >= msg.full_text.chars().count() {
            continue;
        }
        msg.timer.tick(time.delta());
        if msg.timer.just_finished() {
            msg.shown_chars += 1;
            let shown: String = msg.full_text.chars().take(msg.shown_chars).collect();
            *text = Text::new(shown);
        }
    }
}

/// 重ね描き用テキストの一区間。
/// `visible`でない区間も本文と同じ文字を透明色で描くことで、
/// 文字幅と折り返し位置を本文と完全に一致させる（空白で置き換えると全角文字の桁が詰まってずれる）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlaySegment {
    pub text: String,
    pub visible: bool,
}

/// 重ね描きノードに現在描いている区間。変化があったときだけ子の`TextSpan`を作り直すために保持する。
#[derive(Component, Default)]
pub struct OverlayState(Vec<OverlaySegment>);

/// 本文を`visible`の真偽が同じ連続区間に分け、各文字を`map`で置き換えた区間列を作る。
fn split_overlay(
    text: &str,
    visible: impl Fn(usize) -> bool,
    map: impl Fn(char, bool) -> char,
) -> Vec<OverlaySegment> {
    let mut segments: Vec<OverlaySegment> = Vec::new();
    for (i, c) in text.chars().enumerate() {
        let v = c != '\n' && visible(i);
        let mapped = if c == '\n' { c } else { map(c, v) };
        match segments.last_mut() {
            Some(last) if last.visible == v => last.text.push(mapped),
            _ => segments.push(OverlaySegment {
                text: mapped.to_string(),
                visible: v,
            }),
        }
    }
    segments
}

/// 各`LearnableSpan`の文字だけを下線記号に置き換えて見せる区間列を作る。
/// 下線記号は元の文字と同じ幅のもの（全角なら`＿`、半角なら`_`）を使う。
pub fn build_underline_segments(text: &str, spans: &[LearnableSpan]) -> Vec<OverlaySegment> {
    split_overlay(
        text,
        |i| spans.iter().any(|s| i >= s.start && i < s.end),
        |c, visible| match (visible, c.is_ascii()) {
            (false, _) => c,
            (true, true) => '_',
            (true, false) => '＿',
        },
    )
}

/// `span`が指す対象語の文字だけを見せる区間列を作る。
/// 本文と同じグリッドセルに重ねて別色で描画することで、対象語だけ色替えしたように見せる。
pub fn build_highlight_segments(text: &str, span: &LearnableSpan) -> Vec<OverlaySegment> {
    split_overlay(text, |i| i >= span.start && i < span.end, |c, _| c)
}

/// 重ね描きノードの子`TextSpan`を`segments`で作り直す。
fn apply_overlay(
    commands: &mut Commands,
    entity: Entity,
    root: &mut Mut<Text>,
    state: &mut OverlayState,
    font: &TextFont,
    color: Color,
    segments: Vec<OverlaySegment>,
) {
    // 子の`TextSpan`を入れ替えただけでは再レイアウトされず、古いグリフが残ることがあるため、
    // 親の`Text`も変更済みにして作り直させる。
    root.set_changed();
    let mut node = commands.entity(entity);
    node.despawn_descendants();
    node.with_children(|parent| {
        for segment in &segments {
            parent.spawn((
                TextSpan::new(segment.text.clone()),
                font.clone(),
                TextColor(if segment.visible { color } else { Color::NONE }),
            ));
        }
    });
    state.0 = segments;
}

type OverlayQueryData = (
    Entity,
    &'static mut Text,
    &'static TextFont,
    &'static TextColor,
    &'static mut OverlayState,
);

/// メッセージの文字送りが完了した後にだけ、下線と候補ハイライトのオーバーレイを更新する（ADR-0012）。
/// メッセージ欄に会話の本文（`current_text`）以外が出ているときは描かない。
pub fn dialogue_underline_tick(
    mut commands: Commands,
    mode: Res<AppMode>,
    dialogue_res: Res<ActiveDialogue>,
    typewriter_query: Query<&TypewriterMessage, With<MessageTextNode>>,
    mut underline_query: Query<
        OverlayQueryData,
        (With<MessageUnderlineNode>, Without<MessageHighlightNode>),
    >,
    mut highlight_query: Query<
        OverlayQueryData,
        (With<MessageHighlightNode>, Without<MessageUnderlineNode>),
    >,
) {
    let session = (*mode == AppMode::Dialogue || *mode == AppMode::Inn)
        .then(|| dialogue_res.0.as_ref())
        .flatten();

    // 本文の後ろに通知を連結したメッセージもあるため、前方一致で判定する
    let showing_session_text = session.is_some_and(|session| {
        typewriter_query.iter().any(|t| {
            t.shown_chars >= t.full_text.chars().count()
                && t.full_text.starts_with(&session.current_text)
        })
    });

    let (underline, highlight) = match session {
        Some(session) if showing_session_text => {
            let u = if session.learnable_spans.is_empty() {
                Vec::new()
            } else {
                build_underline_segments(&session.current_text, &session.learnable_spans)
            };
            let h = match session.stage {
                DialogueStage::ChoosingLearnTarget { cursor } => session
                    .learnable_spans
                    .get(cursor)
                    .map(|span| build_highlight_segments(&session.current_text, span))
                    .unwrap_or_default(),
                _ => Vec::new(),
            };
            (u, h)
        }
        _ => (Vec::new(), Vec::new()),
    };

    for (entity, mut root, font, color, mut state) in &mut underline_query {
        if state.0 != underline {
            apply_overlay(
                &mut commands,
                entity,
                &mut root,
                &mut state,
                font,
                color.0,
                underline.clone(),
            );
        }
    }
    for (entity, mut root, font, color, mut state) in &mut highlight_query {
        if state.0 != highlight {
            apply_overlay(
                &mut commands,
                entity,
                &mut root,
                &mut state,
                font,
                color.0,
                highlight.clone(),
            );
        }
    }
}

pub fn monster_flash_tick(
    time: Res<Time>,
    mode: Res<AppMode>,
    mut battle: ResMut<BattleStateRes>,
    mut monster_query: Query<&mut TextColor, With<BattleMonsterTextNode>>,
) {
    if *mode == AppMode::Battle && battle.is_flashing {
        battle.flash_timer.tick(time.delta());
        if let Ok(mut color) = monster_query.get_single_mut() {
            *color = TextColor(palette::DAMAGE);
        }
        if battle.flash_timer.finished() {
            battle.is_flashing = false;
            battle.flash_timer.reset();
            if let Ok(mut color) = monster_query.get_single_mut() {
                *color = TextColor(palette::TEXT);
            }
        }
    }
}

pub fn update_message_window(
    mut events: EventReader<ShowMessage>,
    mut query: Query<(&mut TypewriterMessage, &mut Text), With<MessageTextNode>>,
) {
    for event in events.read() {
        for (mut type_msg, mut text) in &mut query {
            type_msg.full_text = event.0.clone();
            type_msg.shown_chars = 0;
            type_msg.timer.reset();
            *text = Text::new("");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joined(segments: &[OverlaySegment]) -> String {
        segments.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn underline_keeps_text_and_replaces_only_span() {
        let text = "衛兵「封印の祭壇だ」\nok key";
        // 「封印の祭壇」と "key"
        let spans = [
            LearnableSpan { start: 3, end: 8 },
            LearnableSpan { start: 14, end: 17 },
        ];
        let segments = build_underline_segments(text, &spans);
        assert_eq!(joined(&segments), "衛兵「＿＿＿＿＿だ」\nok ___");
        assert_eq!(
            segments
                .iter()
                .filter(|s| s.visible)
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>(),
            vec!["＿＿＿＿＿", "___"]
        );
        // 文字数は本文と一致する（同じ文字幅・同じ折り返しになる）
        assert_eq!(joined(&segments).chars().count(), text.chars().count());
    }

    #[test]
    fn highlight_keeps_all_chars_and_shows_only_span() {
        let text = "衛兵「封印の祭壇だ」";
        let segments = build_highlight_segments(text, &LearnableSpan { start: 3, end: 8 });
        assert_eq!(joined(&segments), text);
        assert_eq!(
            segments,
            vec![
                OverlaySegment {
                    text: "衛兵「".into(),
                    visible: false
                },
                OverlaySegment {
                    text: "封印の祭壇".into(),
                    visible: true
                },
                OverlaySegment {
                    text: "だ」".into(),
                    visible: false
                },
            ]
        );
    }
}
