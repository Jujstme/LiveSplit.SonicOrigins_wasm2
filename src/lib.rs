#![no_std]
#![feature(type_alias_impl_trait, const_async_blocks)]
#![warn(
    clippy::complexity,
    clippy::correctness,
    clippy::perf,
    clippy::style,
    clippy::undocumented_unsafe_blocks,
    rust_2018_idioms
)]

use asr::{
    file_format::pe,
    future::{next_tick, retry},
    signature::Signature,
    time::Duration,
    timer::{self, TimerState},
    watcher::Watcher,
    Address, Address64, Process,
};

mod rtti;
use rtti::Rtti;
mod sonic1;
mod sonic2;
mod sonic3;
mod soniccd;

asr::panic_handler!();
asr::async_main!(nightly);

const PROCESS_NAMES: &[&str] = &["SonicOrigins.exe"];

async fn main() {
    let settings = Settings::register();

    loop {
        // Hook to the target process
        let process = retry(|| PROCESS_NAMES.iter().find_map(|&name| Process::attach(name))).await;

        process
            .until_closes(async {
                // Once the target has been found and attached to, set up some default watchers
                let mut watchers = Watchers::default();

                // Perform memory scanning to look for the addresses we need
                let mut addresses = retry(|| Addresses::init(&process)).await;

                loop {
                    // Splitting logic. Adapted from OG LiveSplit:
                    // Order of execution
                    // 1. update() will always be run first. There are no conditions on the execution of this action.
                    // 2. If the timer is currently either running or paused, then the isLoading, gameTime, and reset actions will be run.
                    // 3. If reset does not return true, then the split action will be run.
                    // 4. If the timer is currently not running (and not paused), then the start action will be run.
                    update_loop(&process, &mut addresses, &mut watchers);

                    let timer_state = timer::state();
                    if timer_state == TimerState::Running || timer_state == TimerState::Paused {
                        if let Some(is_loading) = is_loading(&watchers, &settings) {
                            if is_loading {
                                timer::pause_game_time()
                            } else {
                                timer::resume_game_time()
                            }
                        }

                        if let Some(game_time) = game_time(&watchers, &settings, &addresses) {
                            timer::set_game_time(game_time)
                        }

                        if reset(&watchers, &settings) {
                            timer::reset()
                        } else if split(&watchers, &settings) {
                            timer::split()
                        }
                    }

                    if timer::state() == TimerState::NotRunning && start(&mut watchers, &settings) {
                        timer::start();
                        timer::pause_game_time();

                        if let Some(is_loading) = is_loading(&watchers, &settings) {
                            if is_loading {
                                timer::pause_game_time()
                            } else {
                                timer::resume_game_time()
                            }
                        }
                    }

                    next_tick().await;
                }
            })
            .await;
    }
}

#[derive(asr::user_settings::Settings)]
struct Settings {
    #[default = false]
    /// ---------- STORY MODE ----------
    _start: bool,
    #[default = true]
    /// --> Enable auto start
    story_start: bool,
    #[default = true]
    /// Sonic 1 - Green Hill Zone - Act 1
    story_s1_green_hill_1: bool,
    #[default = true]
    /// Sonic 1 - Green Hill Zone - Act 2
    story_s1_green_hill_2: bool,
    #[default = true]
    /// Sonic 1 - Green Hill Zone - Act 3
    story_s1_green_hill_3: bool,
    #[default = true]
    /// Sonic 1 - Marble Zone - Act 1
    story_s1_marble_1: bool,
    #[default = true]
    /// Sonic 1 - Marble Zone - Act 2
    story_s1_marble_2: bool,
    #[default = true]
    /// Sonic 1 - Marble Zone - Act 3
    story_s1_marble_3: bool,
    #[default = true]
    /// Sonic 1 - Spring Yard Zone - Act 1
    story_s1_spring_yard_1: bool,
    #[default = true]
    /// Sonic 1 - Spring Yard Zone - Act 2
    story_s1_spring_yard_2: bool,
    #[default = true]
    /// Sonic 1 - Spring Yard Zone - Act 3
    story_s1_spring_yard_3: bool,
    #[default = true]
    /// Sonic 1 - Labyrinth Zone - Act 1
    story_s1_labyrinth_1: bool,
    #[default = true]
    /// Sonic 1 - Labyrinth Zone - Act 2
    story_s1_labyrinth_2: bool,
    #[default = true]
    /// Sonic 1 - Labyrinth Zone - Act 3
    story_s1_labyrinth_3: bool,
    #[default = true]
    /// Sonic 1 - Star Light Zone - Act 1
    story_s1_star_light_1: bool,
    #[default = true]
    /// Sonic 1 - Star Light Zone - Act 2
    story_s1_star_light_2: bool,
    #[default = true]
    /// Sonic 1 - Star Light Zone - Act 3
    story_s1_star_light_3: bool,
    #[default = true]
    /// Sonic 1 - Scrap Brain Zone - Act 1
    story_s1_scrap_brain_1: bool,
    #[default = true]
    /// Sonic 1 - Scrap Brain Zone - Act 2
    story_s1_scrap_brain_2: bool,
    #[default = true]
    /// Sonic 1 - Scrap Brain Zone - Act 3
    story_s1_scrap_brain_3: bool,
    #[default = true]
    /// Sonic 1 - Final zone
    story_s1_final_zone: bool,
    #[default = true]
    /// Sonic CD - Palmtree Panic - Act 1
    story_scd_palmtree_panic_1: bool,
    #[default = true]
    /// Sonic CD - Palmtree Panic - Act 2
    story_scd_palmtree_panic_2: bool,
    #[default = true]
    /// Sonic CD - Palmtree Panic - Act 3
    story_scd_palmtree_panic_3: bool,
    #[default = true]
    /// Sonic CD - Collision Chaos - Act 1
    story_scd_collision_chaos_1: bool,
    #[default = true]
    /// Sonic CD - Collision Chaos - Act 2
    story_scd_collision_chaos_2: bool,
    #[default = true]
    /// Sonic CD - Collision Chaos - Act 3
    story_scd_collision_chaos_3: bool,
    #[default = true]
    /// Sonic CD - Tidal Tempest - Act 1
    story_scd_tidal_tempest_1: bool,
    #[default = true]
    /// Sonic CD - Tidal Tempest - Act 2
    story_scd_tidal_tempest_2: bool,
    #[default = true]
    /// Sonic CD - Tidal Tempest - Act 3
    story_scd_tidal_tempest_3: bool,
    #[default = true]
    /// Sonic CD - Quartz Quadrant - Act 1
    story_scd_quartz_quadrant_1: bool,
    #[default = true]
    /// Sonic CD - Quartz Quadrant - Act 2
    story_scd_quartz_quadrant_2: bool,
    #[default = true]
    /// Sonic CD - Quartz Quadrant - Act 3
    story_scd_quartz_quadrant_3: bool,
    #[default = true]
    /// Sonic CD - Wacky Workbench - Act 1
    story_scd_wacky_workbench_1: bool,
    #[default = true]
    /// Sonic CD - Wacky Workbench - Act 2
    story_scd_wacky_workbench_2: bool,
    #[default = true]
    /// Sonic CD - Wacky Workbench - Act 3
    story_scd_wacky_workbench_3: bool,
    #[default = true]
    /// Sonic CD - Stardust Speedway - Act 1
    story_scd_stardust_speedway_1: bool,
    #[default = true]
    /// Sonic CD - Stardust Speedway - Act 2
    story_scd_stardust_speedway_2: bool,
    #[default = true]
    /// Sonic CD - Stardust Speedway - Act 3
    story_scd_stardust_speedway_3: bool,
    #[default = true]
    /// Sonic CD - Metallic Madness - Act 1
    story_scd_metallic_madness_1: bool,
    #[default = true]
    /// Sonic CD - Metallic Madness - Act 2
    story_scd_metallic_madness_2: bool,
    #[default = true]
    /// Sonic CD - Metallic Madness - Act 3
    story_scd_metallic_madness_3: bool,
    #[default = true]
    /// Sonic 2 - Emerald Hill Zone - Act 1
    story_s2_emerald_hill_1: bool,
    #[default = true]
    /// Sonic 2 - Emerald Hill Zone - Act 2
    story_s2_emerald_hill_2: bool,
    #[default = true]
    /// Sonic 2 - Chemical Plant Zone - Act 1
    story_s2_chemical_plant_1: bool,
    #[default = true]
    /// Sonic 2 - Chemical Plant Zone - Act 2
    story_s2_chemical_plant_2: bool,
    #[default = true]
    /// Sonic 2 - Aquatic Ruin Zone - Act 1
    story_s2_aquatic_ruin_1: bool,
    #[default = true]
    /// Sonic 2 - Aquatic Ruin Zone - Act 2
    story_s2_aquatic_ruin_2: bool,
    #[default = true]
    /// Sonic 2 - Casino Night Zone - Act 1
    story_s2_casino_night_1: bool,
    #[default = true]
    /// Sonic 2 - Casino Night Zone - Act 2
    story_s2_casino_night_2: bool,
    #[default = true]
    /// Sonic 2 - Hill Top Zone - Act 1
    story_s2_hill_top_1: bool,
    #[default = true]
    /// Sonic 2 - Hill Top Zone - Act 2
    story_s2_hill_top_2: bool,
    #[default = true]
    /// Sonic 2 - Mystic Cave Zone - Act 1
    story_s2_mystic_cave_1: bool,
    #[default = true]
    /// Sonic 2 - Mystic Cave Zone - Act 2
    story_s2_mystic_cave_2: bool,
    #[default = true]
    /// Sonic 2 - Oil Ocean Zone - Act 1
    story_s2_oil_ocean_1: bool,
    #[default = true]
    /// Sonic 2 - Oil Ocean Zone - Act 2
    story_s2_oil_ocean_2: bool,
    #[default = true]
    /// Sonic 2 - Metropolis Zone - Act 1
    story_s2_metropolis_1: bool,
    #[default = true]
    /// Sonic 2 - Metropolis Zone - Act 2
    story_s2_metropolis_2: bool,
    #[default = true]
    /// Sonic 2 - Metropolis Zone - Act 3
    story_s2_metropolis_3: bool,
    #[default = true]
    /// Sonic 2 - Sky Chase Zone
    story_s2_sky_chase: bool,
    #[default = true]
    /// Sonic 3 - Wing Fortress Zone
    story_s2_wing_fortress: bool,
    #[default = true]
    /// Sonic 2 - Death Egg Zone
    story_s2_death_egg: bool,
    #[default = true]
    /// Sonic 3&K - Angel Island Zone - Act 1
    story_s3_angel_island_1: bool,
    #[default = true]
    /// Sonic 3&K - Angel Island Zone - Act 2
    story_s3_angel_island_2: bool,
    #[default = true]
    /// Sonic 3&K - Hydrocity Zone - Act 1
    story_s3_hydrocity_1: bool,
    #[default = true]
    /// Sonic 3&K - Hydrocity Zone - Act 2
    story_s3_hydricity_2: bool,
    #[default = true]
    /// Sonic 3&K - Marble Garden Zone - Act 1
    story_s3_marble_garden_1: bool,
    #[default = true]
    /// Sonic 3&K - Marble Garden Zone - Act 2
    story_s3_marble_garden_2: bool,
    #[default = true]
    /// Sonic 3&K - Carnival Night Zone - Act 1
    story_s3_carnival_night_1: bool,
    #[default = true]
    /// Sonic 3&K - Carnival Night Zone - Act 2
    story_s3_carnival_night_2: bool,
    #[default = true]
    /// Sonic 3&K - Ice Cap Zone - Act 1
    story_s3_ice_cap_1: bool,
    #[default = true]
    /// Sonic 3&K - Ice Cap Zone - Act 2
    story_s3_ice_cap_2: bool,
    #[default = true]
    /// Sonic 3&K - Launch Base Zone - Act 1
    story_s3_launch_base_1: bool,
    #[default = true]
    /// Sonic 3&K - Launch Base Zone - Act 2
    story_s3_launch_base_2: bool,
    #[default = true]
    /// Sonic 3&K - Mushroom Hill Zone - Act 1
    story_s3_mushroom_hill_1: bool,
    #[default = true]
    /// Sonic 3&K - Mushroom Hill Zone - Act 2
    story_s3_mushroom_hill_2: bool,
    #[default = true]
    /// Sonic 3&K - Flying Battery Zone - Act 1
    story_s3_flying_battery_1: bool,
    #[default = true]
    /// Sonic 3&K - Flying Battery Zone - Act 2
    story_s3_flying_battery_2: bool,
    #[default = true]
    /// Sonic 3&K - Sandopolis Zone - Act 1
    story_s3_sandopolis_1: bool,
    #[default = true]
    /// Sonic 3&K - Sandopolis Zone - Act 2
    story_s3_sandopolis_2: bool,
    #[default = true]
    /// Sonic 3&K - Lava Reef Zone - Act 1
    story_s3_lava_reef_1: bool,
    #[default = true]
    /// Sonic 3&K - Lava Reef Zone - Act 2
    story_s3_lava_reef_2: bool,
    #[default = true]
    /// Sonic 3&K - Hidden Palace Zone
    story_s3_hidden_palace: bool,
    #[default = true]
    /// Sonic 3&K - Sky Sanctuary Zone
    story_s3_sky_sanctuary: bool,
    #[default = true]
    /// Sonic 3&K - Death Egg Zone - Act 1
    story_s3_death_egg_1: bool,
    #[default = true]
    /// Sonic 3&K - Death Egg Zone - Act 2
    story_s3_death_egg_2: bool,
    #[default = true]
    /// Sonic 3&K - Doomsday Zone
    story_s3_doomsday: bool,
    #[default = false]
    /// ---------- SONIC 1 ----------
    _sonic1: bool,
    #[default = true]
    /// --> Enable auto start
    s1_start: bool,
    #[default = true]
    /// Green Hill Zone - Act 1
    s1_green_hill_1: bool,
    #[default = true]
    /// Green Hill Zone - Act 2
    s1_green_hill_2: bool,
    #[default = true]
    /// Green Hill Zone - Act 3
    s1_green_hill_3: bool,
    #[default = true]
    /// Marble Zone - Act 1
    s1_marble_1: bool,
    #[default = true]
    /// Marble Zone - Act 2
    s1_marble_2: bool,
    #[default = true]
    /// Marble Zone - Act 3
    s1_marble_3: bool,
    #[default = true]
    /// Spring Yard Zone - Act 1
    s1_spring_yard_1: bool,
    #[default = true]
    /// Spring Yard Zone - Act 2
    s1_spring_yard_2: bool,
    #[default = true]
    /// Spring Yard Zone - Act 3
    s1_spring_yard_3: bool,
    #[default = true]
    /// Labyrinth Zone - Act 1
    s1_labyrinth_1: bool,
    #[default = true]
    /// Labyrinth Zone - Act 2
    s1_labyrinth_2: bool,
    #[default = true]
    /// Labyrinth Zone - Act 3
    s1_labyrinth_3: bool,
    #[default = true]
    /// Star Light Zone - Act 1
    s1_star_light_1: bool,
    #[default = true]
    /// Star Light Zone - Act 2
    s1_star_light_2: bool,
    #[default = true]
    /// Star Light Zone - Act 3
    s1_star_light_3: bool,
    #[default = true]
    /// Scrap Brain Zone - Act 1
    s1_scrap_brain_1: bool,
    #[default = true]
    /// Scrap Brain Zone - Act 2
    s1_scrap_brain_2: bool,
    /// Scrap Brain Zone - Act 3
    s1_scrap_brain_3: bool,
    #[default = true]
    /// Final Zone
    s1_final_zone: bool,
    #[default = false]
    /// ---------- SONIC CD ----------
    _soniccd: bool,
    #[default = true]
    /// --> Enable auto start
    scd_start: bool,
    #[default = true]
    /// Palmtree Panic Zone - Act 1
    scd_palmtree_panic_1: bool,
    #[default = true]
    /// Palmtree Panic Zone - Act 2
    scd_palmtree_panic_2: bool,
    #[default = true]
    /// Palmtree Panic Zone - Act 3
    scd_palmtree_panic_3: bool,
    #[default = true]
    /// Collision Chaos Zone - Act 1
    scd_collision_chaos_1: bool,
    #[default = true]
    /// Collision Chaos Zone - Act 2
    scd_collision_chaos_2: bool,
    #[default = true]
    /// Collision Chaos Zone - Act 3
    scd_collision_chaos_3: bool,
    #[default = true]
    /// Tidal Tempest Zone - Act 1
    scd_tidal_tempest_1: bool,
    #[default = true]
    /// Tidal Tempest Zone - Act 2
    scd_tidal_tempest_2: bool,
    #[default = true]
    /// Tidal Tempest Zone - Act 3
    scd_tidal_tempest_3: bool,
    #[default = true]
    /// Quartz Quadrant Zone - Act 1
    scd_quartz_quadrant_1: bool,
    #[default = true]
    /// Quartz Quadrant Zone - Act 2
    scd_quartz_quadrant_2: bool,
    #[default = true]
    /// Quartz Quadrant Zone - Act 3
    scd_quartz_quadrant_3: bool,
    #[default = true]
    /// Wacky Workbench Zone - Act 1
    scd_wacky_workbench_1: bool,
    #[default = true]
    /// Wacky Workbench Zone - Act 2
    scd_wacky_workbench_2: bool,
    #[default = true]
    /// Wacky Workbench Zone - Act 3
    scd_wacky_workbench_3: bool,
    #[default = true]
    /// Stardust Speedway Zone - Act 1
    scd_stardust_speedway_1: bool,
    #[default = true]
    /// Stardust Speedway Zone - Act 2
    scd_stardust_speedway_2: bool,
    #[default = true]
    /// Stardust Speedway Zone - Act 3
    scd_stardust_speedway_3: bool,
    #[default = true]
    /// Metallic Madness Zone - Act 1
    scd_metallic_madness_1: bool,
    #[default = true]
    /// Metallic Madness Zone - Act 2
    scd_metallic_madness_2: bool,
    #[default = true]
    /// Metallic Madness Zone - Act 3
    scd_metallic_madness_3: bool,
    #[default = false]
    /// ---------- SONIC 2 ----------
    _sonic2: bool,
    #[default = true]
    /// --> Enable auto start
    s2_start: bool,
    #[default = true]
    /// Emerald Hill Zone - Act 1
    s2_emerald_hill_1: bool,
    #[default = true]
    /// Emerald Hill Zone - Act 2
    s2_emerald_hill_2: bool,
    #[default = true]
    /// Chemical Plant Zone - Act 1
    s2_chemical_plant_1: bool,
    #[default = true]
    /// Chemical Plant Zone - Act 2
    s2_chemical_plant_2: bool,
    #[default = true]
    /// Aquatic Ruin Zone - Act 1
    s2_aquatic_ruin_1: bool,
    #[default = true]
    /// Aquatic Ruin Zone - Act 2
    s2_aquatic_ruin_2: bool,
    #[default = true]
    /// Casino Night Zone - Act 1
    s2_casino_night_1: bool,
    #[default = true]
    /// Casino Night Zone - Act 2
    s2_casino_night_2: bool,
    #[default = true]
    /// Hill Top Zone - Act 1
    s2_hill_top_1: bool,
    #[default = true]
    /// Hill Top Zone - Act 2
    s2_hill_top_2: bool,
    #[default = true]
    /// Mystic Cave Zone - Act 1
    s2_mystic_cave_1: bool,
    #[default = true]
    /// Mystic Cave Zone - Act 2
    s2_mystic_cave_2: bool,
    #[default = true]
    /// Oil Ocean Zone - Act 1
    s2_oil_ocean_1: bool,
    #[default = true]
    /// Oil Ocean Zone - Act 2
    s2_oil_ocean_2: bool,
    /// Metropolis Zone - Act 1
    s2_metropolis_1: bool,
    #[default = true]
    /// Metropolis Zone - Act 2
    s2_metropolis_2: bool,
    #[default = true]
    /// Metropolis Zone - Act 3
    s2_metropolis_3: bool,
    #[default = true]
    /// Sky Chase Zone
    s2_sky_chase: bool,
    #[default = true]
    /// Wing Fortress Zone
    s2_wing_fortress: bool,
    #[default = true]
    /// Death Egg Zone
    s2_death_egg: bool,
    #[default = false]
    /// ---------- SONIC 3&K ----------
    _sonic3: bool,
    #[default = true]
    /// --> Enable auto start
    s3_start: bool,
    #[default = true]
    /// Angel Island Zone - Act 1
    s3_angel_island_1: bool,
    #[default = true]
    /// Angel Island Zone - Act 2
    s3_angel_island_2: bool,
    #[default = true]
    /// Hydrocity Zone - Act 1
    s3_hydrocity_1: bool,
    #[default = true]
    /// Hydrocity Zone - Act 2
    s3_hydrocity_2: bool,
    #[default = true]
    /// Marble Garden Zone - Act 1
    s3_marble_garden_1: bool,
    #[default = true]
    /// Marble Garden Zone - Act 2
    s3_marble_garden_2: bool,
    #[default = true]
    /// Carnival Night Zone - Act 1
    s3_carnival_night_1: bool,
    #[default = true]
    /// Carnival Night Zone - Act 2
    s3_carnival_night_2: bool,
    #[default = true]
    /// Ice Cap Zone - Act 1
    s3_ice_cap_1: bool,
    #[default = true]
    /// Ice Cap Zone - Act 2
    s3_ice_cap_2: bool,
    #[default = true]
    /// Launch Base Zone - Act 1
    s3_launch_base_1: bool,
    #[default = true]
    /// Launch Base Zone - Act 2
    s3_launch_base_2: bool,
    #[default = true]
    /// Mushroom Hill Zone - Act 1
    s3_mushroom_hill_1: bool,
    #[default = true]
    /// Mushroom Hill Zone - Act 2
    s3_mushroom_hill_2: bool,
    #[default = true]
    /// Flying Battery Zone - Act 1
    s3_flying_battery_1: bool,
    #[default = true]
    /// Flying Battery Zone - Act 2
    s3_flying_battery_2: bool,
    #[default = true]
    /// Sandopolis Zone - Act 1
    s3_sandopolis_1: bool,
    #[default = true]
    /// Sandopolis Zone - Act 2
    s3_sandopolis_2: bool,
    #[default = true]
    /// Lava Reef Zone - Act 1
    s3_lava_reef_1: bool,
    #[default = true]
    /// Leve Reef Zone - Act 2
    s3_lava_reef_2: bool,
    #[default = true]
    /// Hidden Palace Zone
    s3_hidden_palace: bool,
    #[default = true]
    /// Sky Sanctuary Zone
    s3_sky_sanctuary: bool,
    #[default = true]
    /// Death Egg Zone - Act 1
    s3_death_egg_1: bool,
    #[default = true]
    /// Death Egg Zone - Act 2
    s3_death_egg_2: bool,
    #[default = true]
    /// Doomsday Zone
    s3_doomsday: bool,
}

#[derive(Default)]
struct Watchers {
    game_status: Watcher<GameStatus>,
    game: Watcher<Game>,
    game_mode: Watcher<GameMode>,
    act_id: Watcher<LevelID>,
    start_trigger: Watcher<bool>,
    is_in_time_bonus: Watcher<bool>,
    demo_mode: Watcher<bool>,
    story_start_flag: bool,
}

struct Addresses {
    hedgehog_base: Address,
    current_rsdk_game: Address,
    managers: Managers,
    rtti: Rtti,
}

impl Addresses {
    fn init(game: &Process) -> Option<Self> {
        let main_module_base = PROCESS_NAMES
            .iter()
            .find_map(|p| game.get_module_address(p).ok())?;
        let main_module_size = pe::read_size_of_image(game, main_module_base)? as u64;
        let main_module_range = (main_module_base, main_module_size);

        let hedgehog_base = {
            const SIG: Signature<9> = Signature::new("E8 ?? ?? ?? ?? 44 39 75 48");
            let ptr = SIG.scan_process_range(game, main_module_range)? + 1;
            let temp_addr = ptr + 0x4 + game.read::<i32>(ptr).ok()? + 0x3;
            temp_addr + game.read::<i32>(temp_addr).ok()? + 0x4
        };

        let current_rsdk_game = {
            const SIG: Signature<14> = Signature::new("89 0D ?? ?? ?? ?? 89 15 ?? ?? ?? ?? C7 05");
            let ptr = SIG.scan_process_range(game, main_module_range)? + 2;
            ptr + 0x4 + game.read::<i32>(ptr).ok()?
        };

        Some(Self {
            hedgehog_base,
            current_rsdk_game,
            managers: Managers::new(game, main_module_range)?,
            rtti: Rtti::new(main_module_base),
        })
    }
}

struct Managers {
    sonic_1: sonic1::Sonic1,
    sonic_2: sonic2::Sonic2,
    sonic_cd: soniccd::SonicCD,
    sonic_3: sonic3::Sonic3,
}

impl Managers {
    fn new(process: &Process, main_module_range: (Address, u64)) -> Option<Self> {
        Some(Self {
            sonic_1: sonic1::Sonic1::new(process, main_module_range)?,
            sonic_2: sonic2::Sonic2::new(process, main_module_range)?,
            sonic_cd: soniccd::SonicCD::new(process, main_module_range)?,
            sonic_3: sonic3::Sonic3::new(process, main_module_range)?,
        })
    }
}

fn update_loop(game: &Process, addresses: &mut Addresses, watchers: &mut Watchers) {
    let game_status = watchers.game_status.update_infallible({
        let current = match watchers.game_status.pair {
            Some(x) => x.current,
            _ => GameStatus::MainMenu,
        };
        if let Ok(addr) = game
            .read_pointer_path64::<Address64>(addresses.hedgehog_base, &[0, 0x88, 0x0, 0x70, 0x0])
        {
            if let Some(name) = addresses.rtti.lookup(game, addr.into()) {
                match name.as_bytes() {
                    b"GameModeMainMenu@game@app@@" => GameStatus::MainMenu,
                    b"GameModeRetroEngine@game@app@@" => GameStatus::RetroEngine,
                    b"GameModeGameGear@game@app@@" => GameStatus::GameGear,
                    _ => current,
                }
            } else {
                current
            }
        } else {
            current
        }
    });

    let cur_game = watchers.game.update_infallible(match game_status.current {
        GameStatus::RetroEngine => match game
            .read::<u8>(addresses.current_rsdk_game)
            .unwrap_or_default()
        {
            0 => Game::Sonic1,
            1 => Game::Sonic2,
            2 => Game::Sonic3,
            3 => Game::SonicCD,
            _ => match watchers.game.pair {
                Some(x) => x.current,
                _ => Game::Sonic1,
            },
        },
        GameStatus::MainMenu | GameStatus::GameGear => Game::None,
    });

    watchers
        .game_mode
        .update_infallible(match cur_game.current {
            Game::Sonic1 => addresses.managers.sonic_1.get_game_mode(game),
            Game::Sonic2 => addresses.managers.sonic_2.get_game_mode(game),
            Game::SonicCD => addresses.managers.sonic_cd.get_game_mode(game),
            Game::Sonic3 => addresses.managers.sonic_3.get_game_mode(game),
            _ => GameMode::Classic,
        });

    let act_id = watchers.act_id.update_infallible(match cur_game.current {
        Game::Sonic1 => addresses.managers.sonic_1.get_current_level(game),
        Game::Sonic2 => addresses.managers.sonic_2.get_current_level(game),
        Game::SonicCD => addresses.managers.sonic_cd.get_current_level(game),
        Game::Sonic3 => addresses.managers.sonic_3.get_current_level(game),
        _ => LevelID::MainMenu,
    });

    watchers
        .start_trigger
        .update_infallible(match cur_game.current {
            Game::Sonic1 => addresses.managers.sonic_1.get_start_trigger(game),
            Game::Sonic2 => addresses.managers.sonic_2.get_start_trigger(game),
            Game::SonicCD => addresses.managers.sonic_cd.get_start_trigger(game),
            Game::Sonic3 => addresses.managers.sonic_3.get_start_trigger(game),
            _ => false,
        });

    watchers
        .is_in_time_bonus
        .update_infallible(match cur_game.current {
            Game::Sonic1 => addresses.managers.sonic_1.is_in_time_bonus(game),
            Game::Sonic2 => addresses.managers.sonic_2.is_in_time_bonus(game),
            Game::SonicCD => addresses.managers.sonic_cd.is_in_time_bonus(game),
            Game::Sonic3 => addresses.managers.sonic_3.is_in_time_bonus(),
            _ => false,
        });

    watchers
        .demo_mode
        .update_infallible(match cur_game.current {
            Game::Sonic1 => addresses.managers.sonic_1.is_demo_mode(game),
            Game::Sonic2 => addresses.managers.sonic_2.is_demo_mode(game),
            Game::SonicCD => addresses.managers.sonic_cd.is_demo_mode(game),
            Game::Sonic3 => addresses.managers.sonic_3.is_demo_mode(),
            _ => false,
        });

    if act_id.current == LevelID::MainMenu {
        watchers.story_start_flag = true;
    }
}

fn start(watchers: &mut Watchers, settings: &Settings) -> bool {
    let Some(game) = &watchers.game.pair else {
        return false;
    };
    let Some(game_mode) = &watchers.game_mode.pair else {
        return false;
    };
    let Some(start_trigger) = &watchers.start_trigger.pair else {
        return false;
    };

    if watchers.story_start_flag
        && watchers
            .act_id
            .pair
            .is_some_and(|val| val.current == LevelID::Sonic1_GreenHillAct1)
        && game_mode.current == GameMode::Story
    {
        watchers.story_start_flag = false;
        settings.story_start
    } else if game.current == Game::Sonic1
        && (game_mode.current == GameMode::Classic
            || game_mode.current == GameMode::Anniversary
            || game_mode.current == GameMode::Mirror)
        && start_trigger.changed_to(&true)
    {
        settings.s1_start
    } else if game.current == Game::SonicCD
        && (game_mode.current == GameMode::Classic
            || game_mode.current == GameMode::Anniversary
            || game_mode.current == GameMode::Mirror)
        && start_trigger.changed_to(&true)
    {
        settings.scd_start
    } else if game.current == Game::Sonic2
        && (game_mode.current == GameMode::Classic
            || game_mode.current == GameMode::Anniversary
            || game_mode.current == GameMode::Mirror)
        && start_trigger.changed_to(&true)
    {
        settings.s2_start
    } else if game.current == Game::Sonic3
        && (game_mode.current == GameMode::Classic
            || game_mode.current == GameMode::Anniversary
            || game_mode.current == GameMode::Mirror)
        && start_trigger.changed_to(&true)
    {
        settings.s3_start
    } else {
        false
    }
}

fn split(watchers: &Watchers, settings: &Settings) -> bool {
    if watchers.demo_mode.pair.is_some_and(|val| val.current) {
        return false;
    }
    let Some(game_mode) = &watchers.game_mode.pair else {
        return false;
    };
    let Some(level_id) = &watchers.act_id.pair else {
        return false;
    };

    if game_mode.current == GameMode::Classic
        || game_mode.current == GameMode::Mirror
        || game_mode.current == GameMode::Anniversary
        || game_mode.current == GameMode::Story
    {
        match level_id.old {
            LevelID::Sonic1_GreenHillAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_green_hill_1,
                    _ => settings.s1_green_hill_1,
                }) && level_id.current == LevelID::Sonic1_GreenHillAct2
            }
            LevelID::Sonic1_GreenHillAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_green_hill_2,
                    _ => settings.s1_green_hill_2,
                }) && level_id.current == LevelID::Sonic1_GreenHillAct3
            }
            LevelID::Sonic1_GreenHillAct3 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_green_hill_3,
                    _ => settings.s1_green_hill_3,
                }) && level_id.current == LevelID::Sonic1_MarbleAct1
            }
            LevelID::Sonic1_MarbleAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_marble_1,
                    _ => settings.s1_marble_1,
                }) && level_id.current == LevelID::Sonic1_MarbleAct2
            }
            LevelID::Sonic1_MarbleAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_marble_2,
                    _ => settings.s1_marble_2,
                }) && level_id.current == LevelID::Sonic1_MarbleAct2
            }
            LevelID::Sonic1_MarbleAct3 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_marble_3,
                    _ => settings.s1_marble_3,
                }) && level_id.current == LevelID::Sonic1_SpringYardAct1
            }
            LevelID::Sonic1_SpringYardAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_spring_yard_1,
                    _ => settings.s1_spring_yard_1,
                }) && level_id.current == LevelID::Sonic1_SpringYardAct2
            }
            LevelID::Sonic1_SpringYardAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_spring_yard_2,
                    _ => settings.s1_spring_yard_2,
                }) && level_id.current == LevelID::Sonic1_SpringYardAct3
            }
            LevelID::Sonic1_SpringYardAct3 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_spring_yard_3,
                    _ => settings.s1_spring_yard_3,
                }) && level_id.current == LevelID::Sonic1_LabyrinthAct1
            }
            LevelID::Sonic1_LabyrinthAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_labyrinth_1,
                    _ => settings.s1_labyrinth_1,
                }) && level_id.current == LevelID::Sonic1_LabyrinthAct2
            }
            LevelID::Sonic1_LabyrinthAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_labyrinth_2,
                    _ => settings.s1_labyrinth_2,
                }) && level_id.current == LevelID::Sonic1_LabyrinthAct3
            }
            LevelID::Sonic1_LabyrinthAct3 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_labyrinth_3,
                    _ => settings.s1_labyrinth_3,
                }) && level_id.current == LevelID::Sonic1_StarLightAct1
            }
            LevelID::Sonic1_StarLightAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_star_light_1,
                    _ => settings.s1_star_light_1,
                }) && level_id.current == LevelID::Sonic1_StarLightAct2
            }
            LevelID::Sonic1_StarLightAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_star_light_2,
                    _ => settings.s1_star_light_2,
                }) && level_id.current == LevelID::Sonic1_StarLightAct3
            }
            LevelID::Sonic1_StarLightAct3 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_star_light_3,
                    _ => settings.s1_star_light_3,
                }) && level_id.current == LevelID::Sonic1_ScrapBrainAct1
            }
            LevelID::Sonic1_ScrapBrainAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_scrap_brain_1,
                    _ => settings.s1_scrap_brain_1,
                }) && level_id.current == LevelID::Sonic1_ScrapBrainAct2
            }
            LevelID::Sonic1_ScrapBrainAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_scrap_brain_2,
                    _ => settings.s1_scrap_brain_2,
                }) && level_id.current == LevelID::Sonic1_ScrapBrainAct3
            }
            LevelID::Sonic1_ScrapBrainAct3 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_scrap_brain_3,
                    _ => settings.s1_scrap_brain_3,
                }) && level_id.current == LevelID::Sonic1_FinalZone
            }
            LevelID::Sonic1_FinalZone => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s1_final_zone,
                    _ => settings.s1_final_zone,
                }) && level_id.current == LevelID::Sonic1_Ending
            }
            LevelID::Sonic2_EmeraldHillAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_emerald_hill_1,
                    _ => settings.s2_emerald_hill_1,
                }) && level_id.current == LevelID::Sonic2_EmeraldHillAct2
            }
            LevelID::Sonic2_EmeraldHillAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_emerald_hill_2,
                    _ => settings.s2_emerald_hill_2,
                }) && level_id.current == LevelID::Sonic2_ChemicalPlantAct1
            }
            LevelID::Sonic2_ChemicalPlantAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_chemical_plant_1,
                    _ => settings.s2_chemical_plant_1,
                }) && level_id.current == LevelID::Sonic2_ChemicalPlantAct2
            }
            LevelID::Sonic2_ChemicalPlantAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_chemical_plant_2,
                    _ => settings.s2_chemical_plant_2,
                }) && level_id.current == LevelID::Sonic2_AquaticRuinAct1
            }
            LevelID::Sonic2_AquaticRuinAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_aquatic_ruin_1,
                    _ => settings.s2_aquatic_ruin_1,
                }) && level_id.current == LevelID::Sonic2_AquaticRuinAct2
            }
            LevelID::Sonic2_AquaticRuinAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_aquatic_ruin_2,
                    _ => settings.s2_aquatic_ruin_2,
                }) && level_id.current == LevelID::Sonic2_CasinoNightAct1
            }
            LevelID::Sonic2_CasinoNightAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_casino_night_1,
                    _ => settings.s2_casino_night_1,
                }) && level_id.current == LevelID::Sonic2_CasinoNightAct2
            }
            LevelID::Sonic2_CasinoNightAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_casino_night_2,
                    _ => settings.s2_casino_night_2,
                }) && level_id.current == LevelID::Sonic2_HillTopAct1
            }
            LevelID::Sonic2_HillTopAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_hill_top_1,
                    _ => settings.s2_hill_top_1,
                }) && level_id.current == LevelID::Sonic2_HillTopAct2
            }
            LevelID::Sonic2_HillTopAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_hill_top_2,
                    _ => settings.s2_hill_top_2,
                }) && level_id.current == LevelID::Sonic2_MysticCaveAct1
            }
            LevelID::Sonic2_MysticCaveAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_mystic_cave_1,
                    _ => settings.s2_mystic_cave_1,
                }) && level_id.current == LevelID::Sonic2_MysticCaveAct2
            }
            LevelID::Sonic2_MysticCaveAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_mystic_cave_2,
                    _ => settings.s2_mystic_cave_2,
                }) && level_id.current == LevelID::Sonic2_OilOceanAct1
            }
            LevelID::Sonic2_OilOceanAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_oil_ocean_1,
                    _ => settings.s2_oil_ocean_1,
                }) && level_id.current == LevelID::Sonic2_OilOceanAct2
            }
            LevelID::Sonic2_OilOceanAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_oil_ocean_2,
                    _ => settings.s2_oil_ocean_2,
                }) && level_id.current == LevelID::Sonic2_MetropolisAct1
            }
            LevelID::Sonic2_MetropolisAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_metropolis_1,
                    _ => settings.s2_metropolis_1,
                }) && level_id.current == LevelID::Sonic2_MetropolisAct2
            }
            LevelID::Sonic2_MetropolisAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_metropolis_2,
                    _ => settings.s2_metropolis_2,
                }) && level_id.current == LevelID::Sonic2_MetropolisAct3
            }
            LevelID::Sonic2_MetropolisAct3 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_metropolis_3,
                    _ => settings.s2_metropolis_3,
                }) && level_id.current == LevelID::Sonic2_SkyChase
            }
            LevelID::Sonic2_SkyChase => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_sky_chase,
                    _ => settings.s2_sky_chase,
                }) && level_id.current == LevelID::Sonic2_WingFortress
            }
            LevelID::Sonic2_WingFortress => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_wing_fortress,
                    _ => settings.s2_wing_fortress,
                }) && level_id.current == LevelID::Sonic2_DeathEgg
            }
            LevelID::Sonic2_DeathEgg => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s2_death_egg,
                    _ => settings.s2_death_egg,
                }) && level_id.current == LevelID::Sonic2_Ending
            }
            LevelID::Sonic3_AngelIslandAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_angel_island_1,
                    _ => settings.s3_angel_island_1,
                }) && level_id.current == LevelID::Sonic3_AngelIslandAct2
            }
            LevelID::Sonic3_AngelIslandAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_angel_island_2,
                    _ => settings.s3_angel_island_2,
                }) && level_id.current == LevelID::Sonic3_HydrocityAct1
            }
            LevelID::Sonic3_HydrocityAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_hydrocity_1,
                    _ => settings.s3_hydrocity_1,
                }) && level_id.current == LevelID::Sonic3_HydrocityAct2
            }
            LevelID::Sonic3_HydrocityAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_hydricity_2,
                    _ => settings.s3_hydrocity_2,
                }) && level_id.current == LevelID::Sonic3_MarbleGardenAct1
            }
            LevelID::Sonic3_MarbleGardenAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_marble_garden_1,
                    _ => settings.s3_marble_garden_1,
                }) && level_id.current == LevelID::Sonic3_MarbleGardenAct2
            }
            LevelID::Sonic3_MarbleGardenAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_marble_garden_2,
                    _ => settings.s3_marble_garden_2,
                }) && level_id.current == LevelID::Sonic3_CarnivalNightAct1
            }
            LevelID::Sonic3_CarnivalNightAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_carnival_night_1,
                    _ => settings.s3_carnival_night_1,
                }) && level_id.current == LevelID::Sonic3_CarnivalNightAct2
            }
            LevelID::Sonic3_CarnivalNightAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_carnival_night_2,
                    _ => settings.s3_carnival_night_2,
                }) && level_id.current == LevelID::Sonic3_IceCapAct1
            }
            LevelID::Sonic3_IceCapAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_ice_cap_1,
                    _ => settings.s3_ice_cap_1,
                }) && level_id.current == LevelID::Sonic3_IceCapAct2
            }
            LevelID::Sonic3_IceCapAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_ice_cap_2,
                    _ => settings.s3_ice_cap_2,
                }) && level_id.current == LevelID::Sonic3_LaunchBaseAct1
            }
            LevelID::Sonic3_LaunchBaseAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_launch_base_1,
                    _ => settings.s3_launch_base_1,
                }) && level_id.current == LevelID::Sonic3_LaunchBaseAct2
            }
            LevelID::Sonic3_LaunchBaseAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_launch_base_2,
                    _ => settings.s3_launch_base_2,
                }) && level_id.current == LevelID::Sonic3_MushroomHillAct1
            }
            LevelID::Sonic3_MushroomHillAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_mushroom_hill_1,
                    _ => settings.s3_mushroom_hill_1,
                }) && (level_id.current == LevelID::Sonic3_MushroomHillAct2
                    || level_id.current == LevelID::Sonic3_HiddenPalace)
            }
            LevelID::Sonic3_MushroomHillAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_mushroom_hill_2,
                    _ => settings.s3_mushroom_hill_2,
                }) && (level_id.current == LevelID::Sonic3_FlyingBatteryAct1
                    || level_id.current == LevelID::Sonic3_HiddenPalace)
            }
            LevelID::Sonic3_FlyingBatteryAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_flying_battery_1,
                    _ => settings.s3_flying_battery_1,
                }) && (level_id.current == LevelID::Sonic3_FlyingBatteryAct2
                    || level_id.current == LevelID::Sonic3_HiddenPalace)
            }
            LevelID::Sonic3_FlyingBatteryAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_flying_battery_2,
                    _ => settings.s3_flying_battery_2,
                }) && (level_id.current == LevelID::Sonic3_SandopolisAct1
                    || level_id.current == LevelID::Sonic3_HiddenPalace)
            }
            LevelID::Sonic3_SandopolisAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_sandopolis_1,
                    _ => settings.s3_sandopolis_1,
                }) && (level_id.current == LevelID::Sonic3_SandopolisAct2
                    || level_id.current == LevelID::Sonic3_HiddenPalace)
            }
            LevelID::Sonic3_SandopolisAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_sandopolis_2,
                    _ => settings.s3_sandopolis_2,
                }) && (level_id.current == LevelID::Sonic3_LavaReefAct1
                    || level_id.current == LevelID::Sonic3_HiddenPalace)
            }
            LevelID::Sonic3_LavaReefAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_lava_reef_1,
                    _ => settings.s3_lava_reef_1,
                }) && (level_id.current == LevelID::Sonic3_LavaReefAct2
                    || level_id.current == LevelID::Sonic3_HiddenPalace)
            }
            LevelID::Sonic3_LavaReefAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_lava_reef_2,
                    _ => settings.s3_lava_reef_2,
                }) && level_id.current == LevelID::Sonic3_HiddenPalace
            }
            LevelID::Sonic3_HiddenPalace => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_hidden_palace,
                    _ => settings.s3_hidden_palace,
                }) && level_id.current == LevelID::Sonic3_SkySanctuary
            }
            LevelID::Sonic3_SkySanctuary => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_sky_sanctuary,
                    _ => settings.s3_sky_sanctuary,
                }) && (level_id.current == LevelID::Sonic3_DeathEggAct1
                    || level_id.current == LevelID::Sonic3_Ending)
            }
            LevelID::Sonic3_DeathEggAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_death_egg_1,
                    _ => settings.s3_death_egg_1,
                }) && level_id.current == LevelID::Sonic3_DeathEggAct2
            }
            LevelID::Sonic3_DeathEggAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_death_egg_2,
                    _ => settings.s3_death_egg_2,
                }) && (level_id.current == LevelID::Sonic3_Doomsday
                    || level_id.current == LevelID::Sonic3_Ending)
            }
            LevelID::Sonic3_Doomsday => {
                (match game_mode.current {
                    GameMode::Story => settings.story_s3_doomsday,
                    _ => settings.s3_doomsday,
                }) && level_id.current == LevelID::Sonic3_Ending
            }
            LevelID::SonicCD_PalmtreePanicAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_palmtree_panic_1,
                    _ => settings.scd_palmtree_panic_1,
                }) && level_id.current == LevelID::SonicCD_PalmtreePanicAct2
            }
            LevelID::SonicCD_PalmtreePanicAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_palmtree_panic_2,
                    _ => settings.scd_palmtree_panic_2,
                }) && level_id.current == LevelID::SonicCD_PalmtreePanicAct3
            }
            LevelID::SonicCD_PalmtreePanicAct3 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_palmtree_panic_3,
                    _ => settings.scd_palmtree_panic_3,
                }) && level_id.current == LevelID::SonicCD_CollisionChaosAct1
            }
            LevelID::SonicCD_CollisionChaosAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_collision_chaos_1,
                    _ => settings.scd_collision_chaos_1,
                }) && level_id.current == LevelID::SonicCD_CollisionChaosAct2
            }
            LevelID::SonicCD_CollisionChaosAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_collision_chaos_2,
                    _ => settings.scd_collision_chaos_2,
                }) && level_id.current == LevelID::SonicCD_CollisionChaosAct3
            }
            LevelID::SonicCD_CollisionChaosAct3 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_collision_chaos_3,
                    _ => settings.scd_collision_chaos_3,
                }) && level_id.current == LevelID::SonicCD_TidalTempestAct1
            }
            LevelID::SonicCD_TidalTempestAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_tidal_tempest_1,
                    _ => settings.scd_tidal_tempest_1,
                }) && level_id.current == LevelID::SonicCD_TidalTempestAct2
            }
            LevelID::SonicCD_TidalTempestAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_tidal_tempest_2,
                    _ => settings.scd_tidal_tempest_2,
                }) && level_id.current == LevelID::SonicCD_TidalTempestAct3
            }
            LevelID::SonicCD_TidalTempestAct3 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_tidal_tempest_3,
                    _ => settings.scd_tidal_tempest_3,
                }) && level_id.current == LevelID::SonicCD_QuartzQuadrantAct1
            }
            LevelID::SonicCD_QuartzQuadrantAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_quartz_quadrant_1,
                    _ => settings.scd_quartz_quadrant_1,
                }) && level_id.current == LevelID::SonicCD_QuartzQuadrantAct2
            }
            LevelID::SonicCD_QuartzQuadrantAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_quartz_quadrant_2,
                    _ => settings.scd_quartz_quadrant_2,
                }) && level_id.current == LevelID::SonicCD_QuartzQuadrantAct3
            }
            LevelID::SonicCD_QuartzQuadrantAct3 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_quartz_quadrant_3,
                    _ => settings.scd_quartz_quadrant_3,
                }) && level_id.current == LevelID::SonicCD_WackyWorkbenchAct1
            }
            LevelID::SonicCD_WackyWorkbenchAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_wacky_workbench_1,
                    _ => settings.scd_wacky_workbench_1,
                }) && level_id.current == LevelID::SonicCD_WackyWorkbenchAct2
            }
            LevelID::SonicCD_WackyWorkbenchAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_wacky_workbench_2,
                    _ => settings.scd_wacky_workbench_2,
                }) && level_id.current == LevelID::SonicCD_WackyWorkbenchAct3
            }
            LevelID::SonicCD_WackyWorkbenchAct3 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_wacky_workbench_3,
                    _ => settings.scd_wacky_workbench_3,
                }) && level_id.current == LevelID::SonicCD_StardustSpeedwayAct1
            }
            LevelID::SonicCD_StardustSpeedwayAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_stardust_speedway_1,
                    _ => settings.scd_stardust_speedway_1,
                }) && level_id.current == LevelID::SonicCD_StardustSpeedwayAct2
            }
            LevelID::SonicCD_StardustSpeedwayAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_stardust_speedway_2,
                    _ => settings.scd_stardust_speedway_2,
                }) && level_id.current == LevelID::SonicCD_StardustSpeedwayAct3
            }
            LevelID::SonicCD_StardustSpeedwayAct3 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_stardust_speedway_3,
                    _ => settings.scd_stardust_speedway_3,
                }) && level_id.current == LevelID::SonicCD_MetallicMadnessAct1
            }
            LevelID::SonicCD_MetallicMadnessAct1 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_metallic_madness_1,
                    _ => settings.scd_metallic_madness_1,
                }) && level_id.current == LevelID::SonicCD_MetallicMadnessAct2
            }
            LevelID::SonicCD_MetallicMadnessAct2 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_metallic_madness_2,
                    _ => settings.scd_metallic_madness_2,
                }) && level_id.current == LevelID::SonicCD_MetallicMadnessAct3
            }
            LevelID::SonicCD_MetallicMadnessAct3 => {
                (match game_mode.current {
                    GameMode::Story => settings.story_scd_metallic_madness_3,
                    _ => settings.scd_metallic_madness_3,
                }) && level_id.current == LevelID::SonicCD_Ending
            }
            _ => false,
        }
    } else {
        false
    }
}

fn reset(_watchers: &Watchers, _settings: &Settings) -> bool {
    false
}

fn is_loading(watchers: &Watchers, _settings: &Settings) -> Option<bool> {
    Some(watchers.is_in_time_bonus.pair?.current)
}

fn game_time(
    _watchers: &Watchers,
    _settings: &Settings,
    _addresses: &Addresses,
) -> Option<Duration> {
    None
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum GameStatus {
    MainMenu,
    RetroEngine,
    GameGear,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Game {
    None,
    Sonic1,
    SonicCD,
    Sonic2,
    Sonic3,
    //GameGear,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum GameMode {
    Classic,
    Anniversary,
    BossRush,
    Mirror,
    Mission,
    Story,
    BlueSpheresClassic,
    BlueSpheresNew,
}

#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum LevelID {
    MainMenu,
    Unknown,
    Sonic1_TitleScreen,
    Sonic1_GreenHillAct1,
    Sonic1_GreenHillAct2,
    Sonic1_GreenHillAct3,
    Sonic1_MarbleAct1,
    Sonic1_MarbleAct2,
    Sonic1_MarbleAct3,
    Sonic1_SpringYardAct1,
    Sonic1_SpringYardAct2,
    Sonic1_SpringYardAct3,
    Sonic1_LabyrinthAct1,
    Sonic1_LabyrinthAct2,
    Sonic1_LabyrinthAct3,
    Sonic1_StarLightAct1,
    Sonic1_StarLightAct2,
    Sonic1_StarLightAct3,
    Sonic1_ScrapBrainAct1,
    Sonic1_ScrapBrainAct2,
    Sonic1_ScrapBrainAct3,
    Sonic1_FinalZone,
    Sonic1_Ending,
    Sonic2_TitleScreen,
    Sonic2_EmeraldHillAct1,
    Sonic2_EmeraldHillAct2,
    Sonic2_ChemicalPlantAct1,
    Sonic2_ChemicalPlantAct2,
    Sonic2_AquaticRuinAct1,
    Sonic2_AquaticRuinAct2,
    Sonic2_CasinoNightAct1,
    Sonic2_CasinoNightAct2,
    Sonic2_HillTopAct1,
    Sonic2_HillTopAct2,
    Sonic2_MysticCaveAct1,
    Sonic2_MysticCaveAct2,
    Sonic2_OilOceanAct1,
    Sonic2_OilOceanAct2,
    Sonic2_MetropolisAct1,
    Sonic2_MetropolisAct2,
    Sonic2_MetropolisAct3,
    Sonic2_SkyChase,
    Sonic2_WingFortress,
    Sonic2_DeathEgg,
    Sonic2_Ending,
    Sonic3_TitleScreen,
    Sonic3_SaveSelect,
    Sonic3_AngelIslandAct1,
    Sonic3_AngelIslandAct2,
    Sonic3_HydrocityAct1,
    Sonic3_HydrocityAct2,
    Sonic3_MarbleGardenAct1,
    Sonic3_MarbleGardenAct2,
    Sonic3_CarnivalNightAct1,
    Sonic3_CarnivalNightAct2,
    Sonic3_IceCapAct1,
    Sonic3_IceCapAct2,
    Sonic3_LaunchBaseAct1,
    Sonic3_LaunchBaseAct2,
    Sonic3_MushroomHillAct1,
    Sonic3_MushroomHillAct2,
    Sonic3_FlyingBatteryAct1,
    Sonic3_FlyingBatteryAct2,
    Sonic3_SandopolisAct1,
    Sonic3_SandopolisAct2,
    Sonic3_LavaReefAct1,
    Sonic3_LavaReefAct2,
    Sonic3_HiddenPalace,
    Sonic3_SkySanctuary,
    Sonic3_DeathEggAct1,
    Sonic3_DeathEggAct2,
    Sonic3_Doomsday,
    Sonic3_Ending,
    SonicCD_TitleScreen,
    SonicCD_PalmtreePanicAct1,
    SonicCD_PalmtreePanicAct2,
    SonicCD_PalmtreePanicAct3,
    SonicCD_CollisionChaosAct1,
    SonicCD_CollisionChaosAct2,
    SonicCD_CollisionChaosAct3,
    SonicCD_TidalTempestAct1,
    SonicCD_TidalTempestAct2,
    SonicCD_TidalTempestAct3,
    SonicCD_QuartzQuadrantAct1,
    SonicCD_QuartzQuadrantAct2,
    SonicCD_QuartzQuadrantAct3,
    SonicCD_WackyWorkbenchAct1,
    SonicCD_WackyWorkbenchAct2,
    SonicCD_WackyWorkbenchAct3,
    SonicCD_StardustSpeedwayAct1,
    SonicCD_StardustSpeedwayAct2,
    SonicCD_StardustSpeedwayAct3,
    SonicCD_MetallicMadnessAct1,
    SonicCD_MetallicMadnessAct2,
    SonicCD_MetallicMadnessAct3,
    SonicCD_Ending,
}
