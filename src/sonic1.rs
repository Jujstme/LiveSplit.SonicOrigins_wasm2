use crate::{GameMode, LevelID};
use asr::{signature::Signature, watcher::Watcher, Address, Process};

pub struct Sonic1 {
    level_id: Address,
    time_bonus: Address,
    start_trigger: Address,
    demo_mode: Address,
    game_mode: Address,
    level_watcher: Watcher<LevelID>,
    start_trigger_value: Watcher<u8>,
    time_bonus_value: Watcher<u32>,
    time_bonus_start_value: u32,
}

impl Sonic1 {
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
            time_bonus: pointer_path(0, 0, 0x20D0 + 0x28, false),
            start_trigger: pointer_path(0, 0, 0x2418 + 0xD4, false),
            demo_mode: pointer_path(0x4 * 17, 15, 0x5 * 4, true),
            game_mode: pointer_path(0x4 * 17, 15, 0x6B * 4, true),
            level_watcher: Watcher::new(),
            start_trigger_value: Watcher::new(),
            time_bonus_value: Watcher::new(),
            time_bonus_start_value: u32::default(),
        })
    }

    pub fn get_current_level(&mut self, process: &Process) -> LevelID {
        match process.read::<u8>(self.level_id).unwrap_or_default() {
            0 => LevelID::Sonic1_TitleScreen,
            6 => LevelID::Sonic1_GreenHillAct1,
            7 => LevelID::Sonic1_GreenHillAct2,
            8 => LevelID::Sonic1_GreenHillAct3,
            9 => LevelID::Sonic1_MarbleAct1,
            10 => LevelID::Sonic1_MarbleAct2,
            11 => LevelID::Sonic1_MarbleAct3,
            12 => LevelID::Sonic1_SpringYardAct1,
            13 => LevelID::Sonic1_SpringYardAct2,
            14 => LevelID::Sonic1_SpringYardAct3,
            15 => LevelID::Sonic1_LabyrinthAct1,
            16 => LevelID::Sonic1_LabyrinthAct2,
            17 => LevelID::Sonic1_LabyrinthAct3,
            18 => LevelID::Sonic1_StarLightAct1,
            19 => LevelID::Sonic1_StarLightAct2,
            20 => LevelID::Sonic1_StarLightAct3,
            21 => LevelID::Sonic1_ScrapBrainAct1,
            22 => LevelID::Sonic1_ScrapBrainAct2,
            23 => LevelID::Sonic1_ScrapBrainAct3,
            24 => LevelID::Sonic1_FinalZone,
            1 | 2 => LevelID::Sonic1_Ending,
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

        self.time_bonus_start_value != 0 && time_bonus.current != self.time_bonus_start_value
    }

    pub fn get_start_trigger(&mut self, process: &Process) -> bool {
        let start_trigger_value = self
            .start_trigger_value
            .update_infallible(process.read(self.start_trigger).unwrap_or_default());
        start_trigger_value.changed_from_to(&6, &1)
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
