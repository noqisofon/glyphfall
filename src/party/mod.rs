pub mod action;
pub mod diagnosis;
pub mod influence;
pub mod inventory;

pub use action::{evaluate_command, ActionOutcome, PartyCommand};
pub use diagnosis::{diagnose_member, PlayerSkills};
pub use inventory::PlayerInventory;
#[allow(unused_imports)]
pub use influence::{Influence, MentalState, PartyMember, Personality, MAX_NATURAL_INFLUENCE};




#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_influence_natural_cap() {
        let mut inf = Influence::new_natural(50);
        assert_eq!(inf.raw_value, 50);
        assert!(!inf.is_abnormal());

        // 自然加算：89で頭打ちになる
        inf.add_natural(30);
        assert_eq!(inf.raw_value, 80);

        inf.add_natural(30);
        assert_eq!(inf.raw_value, MAX_NATURAL_INFLUENCE);
        assert_eq!(inf.raw_value, 89);
        assert!(!inf.is_abnormal());

        // 最初から95を指定しても89にクランプされる
        let capped = Influence::new_natural(95);
        assert_eq!(capped.raw_value, 89);
    }

    #[test]
    fn test_influence_supernatural() {
        // 魔術・呪術的介入による100超
        let abnormal = Influence::new_supernatural(120);
        assert_eq!(abnormal.raw_value, 120);
        assert!(abnormal.is_abnormal());
    }

    #[test]
    fn test_command_obedience_and_disobedience() {
        // 忠実な戦士（影響度80、基本成功率高）
        let loyal_warrior = PartyMember::new(
            "戦士ガルツ",
            "せんし",
            Influence::new_natural(80),
            MentalState::Normal,
            Personality::Loyal,
        );


        // 遊び人（影響度50、マイナス補正大、サボり率高）
        let slacker = PartyMember::new(
            "遊び人ロロ",
            "あそびにん",
            Influence::new_natural(50),
            MentalState::Normal,
            Personality::Slacker,
        );

        let mut rng = StdRng::seed_from_u64(42);

        // 忠実な戦士は高確率で指示に従う
        let loyal_outcomes: Vec<_> = (0..20)
            .map(|_| evaluate_command(&loyal_warrior, PartyCommand::Attack, &mut rng))
            .collect();
        assert!(loyal_outcomes.iter().any(|o| matches!(o, ActionOutcome::Obeyed { .. })));

        // 遊び人は高確率で不服従・サボりが発生する
        let slacker_outcomes: Vec<_> = (0..20)
            .map(|_| evaluate_command(&slacker, PartyCommand::Attack, &mut rng))
            .collect();
        assert!(slacker_outcomes.iter().any(|o| matches!(o, ActionOutcome::Disobeyed { .. })));
    }

    #[test]
    fn test_diagnose_charmed_vs_yandere() {
        // 熟練者（魔術100, 目星100）：確実に看破
        let expert_skills = PlayerSkills {
            magic_knowledge: 100,
            keen_eye: 100,
        };

        // 素人（魔術0, 目星100）：目星は通るが魔術が通らず混乱する
        let amateur_skills = PlayerSkills {
            magic_knowledge: 0,
            keen_eye: 100,
        };

        // 魅了された騎士
        let charmed_knight = PartyMember::new(
            "騎士アルヴィン",
            "きし",
            Influence::new_supernatural(110),
            MentalState::Charmed,
            Personality::Loyal,
        );

        // ヤンデレの魔法使い（シラフ）
        let yandere_mage = PartyMember::new(
            "魔法使いミレイ",
            "まほうつかい",
            Influence::new_natural(85),
            MentalState::Normal,
            Personality::Yandere,
        );


        let mut rng = StdRng::seed_from_u64(999);

        // 熟練者の観察：魅了を看破
        let report_charmed = diagnose_member(&expert_skills, &charmed_knight, &mut rng);
        assert!(report_charmed.success);
        assert!(report_charmed.conclusion_msg.contains("魅了呪文に操られている"));

        // 熟練者の観察：ヤンデレ（素の執着）を看破
        let report_yandere = diagnose_member(&expert_skills, &yandere_mage, &mut rng);
        assert!(report_yandere.success);
        assert!(report_yandere.conclusion_msg.contains("素の執着"));

        // 素人の観察：ヤンデレを見て魔術異常かどうか判別できず混乱
        let report_confused = diagnose_member(&amateur_skills, &yandere_mage, &mut rng);
        assert!(!report_confused.success);
        assert!(report_confused.conclusion_msg.contains("混乱"));
    }
}
