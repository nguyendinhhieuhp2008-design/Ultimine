//! Ultimine for Horizon: a native mod modelled on FTB Ultimine.
//!
//! Hold sneak while breaking a block. The connected blocks (or a tunnel /
//! square, see `engine::Shape`) break with it, each as the player's own break
//! through `PlayerRef::destroy_block`, so tool wear, loot, XP and exhaustion
//! all follow vanilla. A `busy` guard stops our own breaks from re-expanding.
//!
//! STATUS: `engine.rs` is unit-tested. The calls to `mod_api` below follow the
//! modding book's vein-miner example; names marked `// VERIFY` were inferred
//! from the host trait and must be checked with `cargo horizon-mod check`.
mod engine;

use engine::{Face, Limits, Shape, plan};
use mod_api::{self, BreakBlockEvent, Flow, Hooks, ServerHost};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

struct Ultimine {
    busy: AtomicBool,
    shape: Mutex<Shape>,
}

impl Default for Ultimine {
    fn default() -> Self {
        // Shape picked once at start: HORIZON_ULTIMINE_SHAPE=large_tunnel etc.
        let shape = std::env::var("HORIZON_ULTIMINE_SHAPE")
            .ok()
            .and_then(|s| Shape::parse(&s))
            .unwrap_or(Shape::Shapeless);
        Ultimine { busy: AtomicBool::new(false), shape: Mutex::new(shape) }
    }
}

#[mod_api::horizon_mod]
impl Hooks for Ultimine {
    fn break_block(&self, host: &mut dyn ServerHost, event: &mut BreakBlockEvent<'_>) -> Flow {
        // Our own breaks arrive here too: let them through untouched.
        if self.busy.load(Ordering::Relaxed) {
            return Flow::Continue;
        }
        let Some(player) = host.players().acting_player() else {
            return Flow::Continue;
        };
        // Activation: sneak held. Survival-style breaks only.
        if !player.is_secondary_use_active(host).unwrap_or(false) // VERIFY
            || player.is_creative(host).unwrap_or(false)          // VERIFY
        {
            return Flow::Continue;
        }
        let Some(level) = player.level(host) else {
            return Flow::Continue;
        };
        let origin = event.pos;
        // The face the player is looking at: a ray from the eyes, reach 6.
        let face = player
            .pick(host, 6.0) // VERIFY: EntityRef::pick via PlayerRef
            .filter(|hit| !hit.miss && hit.block == origin)
            .map(|hit| to_face(hit.face))
            .unwrap_or(Face::Up);

        // Intern BlockKeys as small ids so the engine stays game-agnostic.
        let keys = std::cell::RefCell::new(Vec::new());
        let key_of = |pos: [i32; 3]| -> Option<usize> {
            if !level.is_loaded(host, pos) { // VERIFY
                return None;
            }
            let block = level.block(host, pos)?; // VERIFY
            if level.is_air(host, pos).unwrap_or(true) { // VERIFY
                return None;
            }
            let mut keys = keys.borrow_mut();
            Some(match keys.iter().position(|k| *k == block) {
                Some(i) => i,
                None => {
                    keys.push(block);
                    keys.len() - 1
                }
            })
        };
        let shape = *self.shape.lock().unwrap();
        let blocks = plan(shape, origin, face, &key_of, Limits::default());
        if blocks.len() <= 1 {
            return Flow::Continue; // nothing extra: vanilla handles the click
        }

        self.busy.store(true, Ordering::Relaxed);
        for pos in blocks {
            // Stops by itself when the tool breaks or a check refuses.
            let _ = player.destroy_block(host, pos, false);
        }
        self.busy.store(false, Ordering::Relaxed);
        // The origin was broken by the loop above.
        Flow::Cancel
    }
}

fn to_face(face: mod_api::BlockFace) -> Face {
    use mod_api::BlockFace as B;
    match face {
        B::Down => Face::Down,
        B::Up => Face::Up,
        B::North => Face::North,
        B::South => Face::South,
        B::West => Face::West,
        B::East => Face::East,
    }
}
