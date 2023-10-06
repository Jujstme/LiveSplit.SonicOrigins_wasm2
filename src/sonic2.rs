use crate::{GameMode, LevelID};
use asr::{signature::Signature, watcher::Watcher, Address, Process};

pub struct Sonic2 {
    level_id: Address,
    time_bonus: Address,
    start_trigger: Address,
    demo_mode: Address,
    game_mode: Address,
    score_tally: Address,
    continue_bonus: Address,

    level_watcher: Watcher<LevelID>,
    start_trigger_value: Watcher<u8>,
    time_bonus_value: Watcher<u32>,
    time_bonus_start_value: u32,
}

impl Sonic2 {
    pub fn new(process: &Process, main_module_range: (Address, u64)) -> Option<Self> {
        let ptr = {
            const PTR: Signature<27> = Signature::new(
                "3D F9 00 00 00 0F 87 ?? ?? ?? ?? 41 8B 8C 84 ?? ?? ?? ?? 49 03 CC FF ?? 41 8B 84",
            );
            let scanned = PTR.scan_process_range(process, main_module_range)? + 15;
            main_module_range.0 + process.read::<i32>(scanned).ok()?
        };

        let lea = {
            const LEA: Signature<11> = Signature::new("EB ?? 8B CF E8 ?? ?? ?? ?? 8B 3D");
            let scanned = LEA.scan_process_range(process, main_module_range)? + 1;
            let temp_addr = scanned + 0x4 + process.read::<u8>(scanned).ok()?;
            temp_addr + 0x4 + process.read::<i32>(temp_addr).ok()?
        };

        let pointer_path = |offset1, offset2, offset3, absolute| {
            if offset1 == 0 {
                lea + offset3
            } else {
                let temp_offset = process.read::<i32>(ptr + offset1).unwrap_or_default();
                let temp_offset_2 = main_module_range.0 + temp_offset + offset2;

                if absolute {
                    main_module_range.0
                        + process.read::<i32>(temp_offset_2).unwrap_or_default()
                        + offset3
                } else {
                    temp_offset_2
                        + 0x4
                        + process.read::<i32>(temp_offset_2).unwrap_or_default()
                        + offset3
                }
            }
        };

        Some(Self {
            level_id: pointer_path(0x4 * 122, 39, 0, false),
            time_bonus: pointer_path(0, 0, 0x20D0 + 0x30, false),
            start_trigger: pointer_path(0, 0, 0x2418 + 0xD4, false),
            demo_mode: pointer_path(0x4 * 17, 15, 0x5 * 4, true),
            game_mode: pointer_path(0x4 * 17, 15, 0x8F * 4, true),
            score_tally: pointer_path(0, 0, 0x20D0 + 0xD4, false),
            continue_bonus: pointer_path(0, 0, 0x20D0 + 0x44, false),
            level_watcher: Watcher::new(),
            start_trigger_value: Watcher::new(),
            time_bonus_value: Watcher::new(),
            time_bonus_start_value: u32::default(),
        })
    }

    pub fn get_current_level(&mut self, process: &Process) -> LevelID {
        match process.read::<u8>(self.level_id).unwrap_or_default() {
            0 => LevelID::Sonic2_TitleScreen,
            1 | 2 => LevelID::Sonic2_Ending,
            6 => LevelID::Sonic2_EmeraldHillAct1,
            7 => LevelID::Sonic2_EmeraldHillAct2,
            8 => LevelID::Sonic2_ChemicalPlantAct1,
            9 => LevelID::Sonic2_ChemicalPlantAct2,
            10 => LevelID::Sonic2_AquaticRuinAct1,
            11 => LevelID::Sonic2_AquaticRuinAct2,
            12 => LevelID::Sonic2_CasinoNightAct1,
            13 => LevelID::Sonic2_CasinoNightAct2,
            14 => LevelID::Sonic2_HillTopAct1,
            15 => LevelID::Sonic2_HillTopAct2,
            16 => LevelID::Sonic2_MysticCaveAct1,
            17 => LevelID::Sonic2_MysticCaveAct2,
            18 => LevelID::Sonic2_OilOceanAct1,
            19 => LevelID::Sonic2_OilOceanAct2,
            20 => LevelID::Sonic2_MetropolisAct1,
            21 => LevelID::Sonic2_MetropolisAct2,
            22 => LevelID::Sonic2_MetropolisAct3,
            23 => LevelID::Sonic2_SkyChase,
            24 => LevelID::Sonic2_WingFortress,
            25 => LevelID::Sonic2_DeathEgg,
            _ => match self.level_watcher.pair {
                Some(x) => x.current,
                _ => LevelID::MainMenu,
            },
        }
    }

    pub fn is_in_time_bonus(&mut self, process: &Process) -> bool {
        let time_bonus = self
            .time_bonus_value
            .update_infallible(process.read(self.time_bonus).unwrap_or_default());

        if time_bonus.changed_from(&0) {
            self.time_bonus_start_value = time_bonus.current;
        } else if time_bonus.current == 0 {
            self.time_bonus_start_value = 0;
        }

        (self.time_bonus_start_value != 0 && time_bonus.current != self.time_bonus_start_value)
            || (process
                .read::<u8>(self.score_tally)
                .is_ok_and(|val| val == 4)
                && process
                    .read::<u8>(self.continue_bonus)
                    .is_ok_and(|val| val != 0))
    }

    pub fn get_start_trigger(&mut self, process: &Process) -> bool {
        let start_trigger_value = self
            .start_trigger_value
            .update_infallible(process.read(self.start_trigger).unwrap_or_default());
        start_trigger_value.changed_from_to(&8, &9)
    }

    pub fn is_demo_mode(&self, process: &Process) -> bool {
        process.read(self.demo_mode).unwrap_or_default()
    }

    pub fn get_game_mode(&self, process: &Process) -> GameMode {
        match process.read::<u8>(self.game_mode).unwrap_or_default() {
            1 => GameMode::Anniversary,
            2 => GameMode::BossRush,
            3 => GameMode::Mirror,
            4 => GameMode::Mission,
            5 => GameMode::Story,
            6 => GameMode::BlueSpheresClassic,
            7 => GameMode::BlueSpheresNew,
            _ => GameMode::Classic,
        }
    }
}
