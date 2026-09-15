use bevy::prelude::*;
use crate::{
    battle::BattleState,
    town::{DialogueLearnStage, LearnableSpan},
    ActiveDialogue, AppMode, ShowMessage,
};
use super::{
    palette, BattleMonsterTextNode, MessageHighlightNode, MessageTextNode,
    MessageUnderlineNode, TypewriterMessage,
};

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

/// テキスト中の各`LearnableSpan`が占める桁位置にだけ`_`を並べた文字列を作る。
/// 改行はそのまま維持し、行ごとの文字数が本文と一致するようにする
/// （同一グリッドセルに重ね描きしたときに、対象語の真下に揃うようにするため）。
pub fn build_underline_text(text: &str, spans: &[LearnableSpan]) -> String {
    text.chars()
        .enumerate()
        .map(|(i, c)| {
            if c == '\n' {
                '\n'
            } else if spans.iter().any(|s| i >= s.start && i < s.end) {
                '_'
            } else {
                ' '
            }
        })
        .collect()
}

/// `span`が指す対象語の文字だけをそのまま残し、それ以外を空白に置き換えた文字列を作る。
/// 本文と同じグリッドセルに重ねて別色で描画することで、対象語だけ色替えしたように見せる。
pub fn build_highlight_text(text: &str, span: &LearnableSpan) -> String {
    text.chars()
        .enumerate()
        .map(|(i, c)| {
            if c == '\n' {
                '\n'
            } else if i >= span.start && i < span.end {
                c
            } else {
                ' '
            }
        })
        .collect()
}

/// メッセージの文字送りが完了した後にだけ、下線と候補ハイライトのオーバーレイを更新する（ADR-0012）。
pub fn dialogue_underline_tick(
    mode: Res<AppMode>,
    dialogue_res: Res<ActiveDialogue>,
    typewriter_query: Query<&TypewriterMessage, With<MessageTextNode>>,
    mut underline_query: Query<
        &mut Text,
        (With<MessageUnderlineNode>, Without<MessageHighlightNode>),
    >,
    mut highlight_query: Query<
        &mut Text,
        (With<MessageHighlightNode>, Without<MessageUnderlineNode>),
    >,
) {
    let (Ok(mut underline_text), Ok(mut highlight_text)) =
        (underline_query.get_single_mut(), highlight_query.get_single_mut())
    else {
        return;
    };

    let session = (*mode == AppMode::Dialogue).then(|| dialogue_res.0.as_ref()).flatten();
    let Some(session) = session else {
        *underline_text = Text::new("");
        *highlight_text = Text::new("");
        return;
    };

    let fully_shown = typewriter_query
        .get_single()
        .map(|t| t.shown_chars >= t.full_text.chars().count())
        .unwrap_or(false);

    if fully_shown && !session.learnable_spans.is_empty() {
        *underline_text = Text::new(build_underline_text(&session.current_text, &session.learnable_spans));
    } else {
        *underline_text = Text::new("");
    }

    *highlight_text = match session.learn_stage {
        DialogueLearnStage::ChoosingLearnTarget { cursor } if fully_shown => session
            .learnable_spans
            .get(cursor)
            .map(|span| Text::new(build_highlight_text(&session.current_text, span)))
            .unwrap_or_else(|| Text::new("")),
        _ => Text::new(""),
    };
}

pub fn monster_flash_tick(
    time: Res<Time>,
    mode: Res<AppMode>,
    mut battle: ResMut<BattleState>,
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
        if let Ok((mut type_msg, mut text)) = query.get_single_mut() {
            type_msg.full_text = event.0.clone();
            type_msg.shown_chars = 0;
            type_msg.timer.reset();
            *text = Text::new("");
        }
    }
}
