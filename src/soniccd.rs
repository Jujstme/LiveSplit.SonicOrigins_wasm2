use crate::{GameMode, LevelID};
use asr::{signature::Signature, watcher::Watcher, Address, Process};

pub struct SonicCD {
    level_id: Address,
    time_bonus: Address,
    start_trigger: Address,
    demo_mode: Address,
    game_mode: Address,
    time_travel: Address,
    level_watcher: Watcher<LevelID>,
    start_trigger_value: Watcher<u8>,
    time_bonus_value: Watcher<u32>,
    time_bonus_start_value: u32,
}

impl SonicCD {
    pub fn new(process: &Process, main_module_range: (Address, u64)) -> Option<Self> {
        let ptr = {
            const PTR: Signature<27> = Signature::new(
                "3D E4 00 00 00 0F 87 ?? ?? ?? ?? 41 8B 8C 84 ?? ?? ?? ?? 49 03 CC FF ?? 41 8B 84",
            );
            let scanned = PTR.scan_process_range(process, main_module_range)? + 15;
            main_module_range.0 + process.read::<i32>(scanned).ok()?
        };

        let lea = {
            const LEA: Signature<8> = Signature::new("EB ?? 45 33 FF 45 85 D2");
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
            level_id: pointer_path(0x4 * 120, 46, 0, false),
            time_bonus: pointer_path(0, 0, 0x814, false),
            start_trigger: pointer_path(0, 0, 0x942, false),
            demo_mode: pointer_path(0x4 * 11, 15, 0x6B * 4, true),
            game_mode: pointer_path(0x4 * 11, 15, 0x77 * 4, true),
            time_travel: pointer_path(0x4 * 11, 15, 0x1E * 4, true),
            level_watcher: Watcher::new(),
            start_trigger_value: Watcher::new(),
            time_bonus_value: Watcher::new(),
            time_bonus_start_value: u32::default(),
        })
    }

    pub fn get_current_level(&mut self, process: &Process) -> LevelID {
        match process.read::<u8>(self.level_id).unwrap_or_default() {
            0 => LevelID::SonicCD_TitleScreen,
            8 => LevelID::SonicCD_Ending,
            13 | 14 | 15 | 16 => LevelID::SonicCD_PalmtreePanicAct1,
            17 | 18 | 19 | 20 => LevelID::SonicCD_PalmtreePanicAct2,
            21 | 22 => LevelID::SonicCD_PalmtreePanicAct3,
            23 | 24 | 25 | 26 => LevelID::SonicCD_CollisionChaosAct1,
            27 | 28 | 29 | 30 => LevelID::SonicCD_CollisionChaosAct2,
            31 | 32 => LevelID::SonicCD_CollisionChaosAct3,
            33 | 34 | 35 | 36 => LevelID::SonicCD_TidalTempestAct1,
            37 | 38 | 39 | 40 => LevelID::SonicCD_TidalTempestAct2,
            41 | 42 => LevelID::SonicCD_TidalTempestAct3,
            43 | 44 | 45 | 46 => LevelID::SonicCD_QuartzQuadrantAct1,
            47 | 48 | 49 | 50 => LevelID::SonicCD_QuartzQuadrantAct2,
            51 | 52 => LevelID::SonicCD_QuartzQuadrantAct3,
            53 | 54 | 55 | 56 => LevelID::SonicCD_WackyWorkbenchAct1,
            57 | 58 | 59 | 60 => LevelID::SonicCD_WackyWorkbenchAct2,
            61 | 62 => LevelID::SonicCD_WackyWorkbenchAct3,
            63 | 64 | 65 | 66 => LevelID::SonicCD_StardustSpeedwayAct1,
            67 | 68 | 69 | 70 => LevelID::SonicCD_StardustSpeedwayAct2,
            71 | 72 => LevelID::SonicCD_StardustSpeedwayAct3,
            73 | 74 | 75 | 76 => LevelID::SonicCD_MetallicMadnessAct1,
            77 | 78 | 79 | 80 => LevelID::SonicCD_MetallicMadnessAct2,
            81 | 82 => LevelID::SonicCD_MetallicMadnessAct3,
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
            || process.read::<u32>(self.time_travel).unwrap_or_default() != 0
    }

    pub fn get_start_trigger(&mut self, process: &Process) -> bool {
        let start_trigger_value = self
            .start_trigger_value
            .update_infallible(process.read(self.start_trigger).unwrap_or_default());
        start_trigger_value.changed_from_to(&11, &2)
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
