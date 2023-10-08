use crate::{GameMode, LevelID};
use asr::{signature::Signature, watcher::Watcher, Address, Process};

pub struct Sonic3 {
    level_id: Address,
    level_id_apparent: Address,
    status: Address,
    game_mode: Address,
    game_mode_offset: u64,
    hpz_flag: Address,
    level_watcher: Watcher<LevelID>,
}

impl Sonic3 {
    pub fn new(process: &Process, main_module_range: (Address, u64)) -> Option<Self> {
        let status = {
            const SIG: Signature<4> = Signature::new("0A C1 88 05");
            let ptr = SIG.scan_process_range(process, main_module_range)? + 4;
            ptr + 0x4 + process.read::<i32>(ptr).ok()?
        };

        let level_id = {
            const SIG: Signature<9> = Signature::new("66 89 05 ?? ?? ?? ?? 3B DF");
            let ptr = SIG.scan_process_range(process, main_module_range)? + 3;
            ptr + 0x4 + process.read::<i32>(ptr).ok()?
        };

        let level_id_apparent = {
            const SIG: Signature<9> = Signature::new("89 15 ?? ?? ?? ?? 48 8B 87");
            let ptr = SIG.scan_process_range(process, main_module_range)? + 2;
            ptr + 0x4 + process.read::<i32>(ptr).ok()?
        };

        let hpz_flag = {
            const SIG: Signature<9> = Signature::new("4C 39 35 ?? ?? ?? ?? 74 05");
            let ptr = SIG.scan_process_range(process, main_module_range)? + 3;
            ptr + 0x4 + process.read::<i32>(ptr).ok()?
        };

        let ptr = {
            const SIG: Signature<25> = Signature::new(
                "41 83 F8 05 0F 85 ?? ?? ?? ?? 83 F9 16 0F 87 ?? ?? ?? ?? 48 63 C1 48 8D 0D",
            );
            let pptr = SIG.scan_process_range(process, main_module_range)? + 32;
            main_module_range.0 + process.read::<i32>(pptr).ok()?
        };
        let pointer_case = |offset1, offset2| {
            let temp_offset = process.read::<i32>(ptr + offset1).unwrap_or_default();
            main_module_range.0 + temp_offset + offset2
        };
        let b_base = pointer_case(0, 3);
        let game_mode = b_base + 0x4 + process.read::<i32>(b_base).ok()?;
        let game_mode_offset = process.read::<u32>(b_base + 0x6).unwrap_or_default() as _;

        Some(Self {
            level_id,
            level_id_apparent,
            status,
            game_mode,
            game_mode_offset,
            hpz_flag,
            level_watcher: Watcher::new(),
        })
    }

    pub fn get_current_level(&mut self, process: &Process) -> LevelID {
        let level = self.level_watcher.update_infallible({
            let act = process.read::<u8>(self.level_id).unwrap_or_default();

            let r_act = match act {
                0 => Some(LevelID::Sonic3_TitleScreen),
                2 => Some(LevelID::Sonic3_SaveSelect),
                5 => Some(LevelID::Sonic3_Ending),
                15 => Some(LevelID::Sonic3_AngelIslandAct1),
                _ => None,
            };

            if let Some(x) = r_act {
                return x;
            }

            let apparent_act = process
                .read::<u8>(self.level_id_apparent)
                .unwrap_or_default();
            let cur_level = match &self.level_watcher.pair {
                Some(x) => x.current,
                _ => LevelID::Sonic3_AngelIslandAct1,
            };

            match apparent_act {
                0 => LevelID::Sonic3_AngelIslandAct1,
                1 => LevelID::Sonic3_AngelIslandAct2,
                2 => {
                    if act == 17 {
                        LevelID::Sonic3_HydrocityAct1
                    } else {
                        cur_level
                    }
                }
                3 => LevelID::Sonic3_HydrocityAct2,
                4 => LevelID::Sonic3_MarbleGardenAct1,
                5 => LevelID::Sonic3_MarbleGardenAct2,
                6 => LevelID::Sonic3_CarnivalNightAct1,
                7 => LevelID::Sonic3_CarnivalNightAct2,
                8 => LevelID::Sonic3_IceCapAct1,
                9 => LevelID::Sonic3_IceCapAct2,
                10 => LevelID::Sonic3_LaunchBaseAct1,
                11 => LevelID::Sonic3_LaunchBaseAct2,
                12 => LevelID::Sonic3_MushroomHillAct1,
                13 => LevelID::Sonic3_MushroomHillAct2,
                14 => LevelID::Sonic3_FlyingBatteryAct1,
                15 => LevelID::Sonic3_FlyingBatteryAct2,
                16 => LevelID::Sonic3_SandopolisAct1,
                17 => LevelID::Sonic3_SandopolisAct2,
                18 => LevelID::Sonic3_LavaReefAct1,
                19 | 20 => LevelID::Sonic3_LavaReefAct2,
                21 => {
                    if process.read::<u8>(self.status).is_ok_and(|val| val != 2) {
                        cur_level
                    } else if let Ok(val) =
                        process.read_pointer_path64::<u8>(self.hpz_flag, &[0x0, 0x4])
                    {
                        if val == 0 {
                            LevelID::Sonic3_HiddenPalace
                        } else {
                            cur_level
                        }
                    } else {
                        cur_level
                    }
                }
                22 | 23 => LevelID::Sonic3_SkySanctuary,
                24 => LevelID::Sonic3_DeathEggAct1,
                25 | 26 => LevelID::Sonic3_DeathEggAct2,
                27 => LevelID::Sonic3_Doomsday,
                _ => cur_level,
            }
        });
        level.current
    }

    pub fn is_in_time_bonus(&self) -> bool {
        false
    }

    pub fn get_start_trigger(&mut self, _process: &Process) -> bool {
        self.level_watcher.pair.is_some_and(|level| {
            level.old == LevelID::Sonic3_SaveSelect
                && level.current == LevelID::Sonic3_AngelIslandAct1
        })
    }

    pub fn is_demo_mode(&self) -> bool {
        false
    }

    pub fn get_game_mode(&self, process: &Process) -> GameMode {
        match process.read_pointer_path64(self.game_mode, &[0x0, self.game_mode_offset]) {
            Ok(1) => GameMode::Anniversary,
            Ok(2) => GameMode::BossRush,
            Ok(3) => GameMode::Mirror,
            Ok(4) => GameMode::Mission,
            Ok(5) => GameMode::Story,
            Ok(6) => GameMode::BlueSpheresClassic,
            Ok(7) => GameMode::BlueSpheresNew,
            _ => GameMode::Classic,
        }
    }
}
