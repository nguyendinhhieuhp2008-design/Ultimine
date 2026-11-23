//! Host-agnostic Ultimine planning: which blocks a single break expands to.
//! No game types here, so it builds and tests with plain `rustc`/`cargo test`.
use std::collections::{BTreeSet, VecDeque};

pub type Pos = [i32; 3];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Face {
    /// Outward normal of the face that was hit.
    pub fn normal(self) -> Pos {
        match self {
            Face::Down => [0, -1, 0],
            Face::Up => [0, 1, 0],
            Face::North => [0, 0, -1],
            Face::South => [0, 0, 1],
            Face::West => [-1, 0, 0],
            Face::East => [1, 0, 0],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Shapeless,
    SmallTunnel,
    LargeTunnel,
    SmallSquare,
    LargeSquare,
}

impl Shape {
    pub const ALL: [Shape; 5] = [
        Shape::Shapeless,
        Shape::SmallTunnel,
        Shape::LargeTunnel,
        Shape::SmallSquare,
        Shape::LargeSquare,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Shape::Shapeless => "shapeless",
            Shape::SmallTunnel => "small_tunnel",
            Shape::LargeTunnel => "large_tunnel",
            Shape::SmallSquare => "small_square",
            Shape::LargeSquare => "large_square",
        }
    }
    pub fn parse(s: &str) -> Option<Shape> {
        Shape::ALL.into_iter().find(|x| x.name() == s.trim())
    }
    pub fn next(self) -> Shape {
        let i = Shape::ALL.iter().position(|x| *x == self).unwrap_or(0);
        Shape::ALL[(i + 1) % Shape::ALL.len()]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_blocks: usize,
    /// Upper bound on positions read, so a plan never scans unboundedly.
    pub max_examined: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { max_blocks: 64, max_examined: 64 * 27 }
    }
}

fn add(a: Pos, b: Pos, k: i32) -> Option<Pos> {
    Some([
        a[0].checked_add(b[0].checked_mul(k)?)?,
        a[1].checked_add(b[1].checked_mul(k)?)?,
        a[2].checked_add(b[2].checked_mul(k)?)?,
    ])
}

/// `key(pos)` is `Some(block id)` for a loaded, breakable block and `None`
/// for air, unbreakable or unloaded positions. The result lists the origin
/// first, then the other blocks, at most `limits.max_blocks` in total.
pub fn plan<K: Copy + PartialEq>(
    shape: Shape,
    origin: Pos,
    face: Face,
    key: &dyn Fn(Pos) -> Option<K>,
    limits: Limits,
) -> Vec<Pos> {
    let Some(origin_key) = key(origin) else {
        return Vec::new();
    };
    if limits.max_blocks == 0 {
        return Vec::new();
    }
    match shape {
        Shape::Shapeless => shapeless(origin, origin_key, key, limits),
        _ => volume(shape, origin, face, key, limits),
    }
}

fn shapeless<K: Copy + PartialEq>(origin: Pos, origin_key: K, key: &dyn Fn(Pos) -> Option<K>, limits: Limits) -> Vec<Pos> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::from([origin]);
    let mut queue = VecDeque::from([origin]);
    while let Some(p) = queue.pop_front() {
        if out.len() >= limits.max_blocks {
            break;
        }
        if key(p) != Some(origin_key) {
            continue;
        }
        out.push(p);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if (dx, dy, dz) == (0, 0, 0) {
                        continue;
                    }
                    if let Some(n) = add(p, [dx, dy, dz], 1)
                        && seen.len() < limits.max_examined
                        && seen.insert(n)
                    {
                        queue.push_back(n);
                    }
                }
            }
        }
    }
    out
}

fn volume<K: Copy + PartialEq>(shape: Shape, origin: Pos, face: Face, key: &dyn Fn(Pos) -> Option<K>, limits: Limits) -> Vec<Pos> {
    let n = face.normal();
    let dir = [-n[0], -n[1], -n[2]]; // into the block
    let (u, v) = match face {
        Face::Up | Face::Down => ([1, 0, 0], [0, 0, 1]),
        Face::North | Face::South => ([1, 0, 0], [0, 1, 0]),
        Face::West | Face::East => ([0, 0, 1], [0, 1, 0]),
    };
    let (r, tunnel) = match shape {
        Shape::SmallTunnel => (0, true),
        Shape::LargeTunnel => (1, true),
        Shape::SmallSquare => (1, false),
        _ => (2, false),
    };
    let section = ((2 * r + 1) * (2 * r + 1)) as usize;
    let depth = if tunnel { limits.max_blocks.div_ceil(section) } else { 1 };
    let mut cells: Vec<(usize, i32, Pos)> = Vec::new();
    let mut examined = 0usize;
    'outer: for d in 0..depth {
        for a in -r..=r {
            for b in -r..=r {
                if examined >= limits.max_examined {
                    break 'outer;
                }
                examined += 1;
                let Some(base) = add(origin, dir, d as i32) else { continue };
                let Some(p) = add(base, u, a).and_then(|p| add(p, v, b)) else { continue };
                if key(p).is_some() {
                    cells.push((d, a.abs() + b.abs(), p));
                }
            }
        }
    }
    cells.sort_by_key(|(d, ring, p)| (*d, *ring, *p));
    let mut out: Vec<Pos> = cells.into_iter().map(|(_, _, p)| p).collect();
    out.truncate(limits.max_blocks);
    if let Some(i) = out.iter().position(|p| *p == origin) {
        out.swap(0, i);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn world(blocks: &[(Pos, u32)]) -> HashMap<Pos, u32> {
        blocks.iter().copied().collect()
    }
    fn run(w: &HashMap<Pos, u32>, shape: Shape, origin: Pos, face: Face, max: usize) -> Vec<Pos> {
        plan(shape, origin, face, &|p| w.get(&p).copied(), Limits { max_blocks: max, max_examined: 100_000 })
    }

    #[test]
    fn shapeless_follows_same_block_including_diagonals() {
        let w = world(&[([0, 0, 0], 1), ([1, 0, 0], 1), ([2, 1, 0], 1), ([5, 5, 5], 1), ([0, 1, 0], 2)]);
        let got: BTreeSet<_> = run(&w, Shape::Shapeless, [0, 0, 0], Face::Up, 64).into_iter().collect();
        assert_eq!(got, BTreeSet::from([[0, 0, 0], [1, 0, 0], [2, 1, 0]]));
    }

    #[test]
    fn origin_is_first_and_cap_applies() {
        let blocks: Vec<_> = (0..20).map(|x| ([x, 0, 0], 1)).collect();
        let w = world(&blocks);
        let got = run(&w, Shape::Shapeless, [7, 0, 0], Face::Up, 5);
        assert_eq!(got.len(), 5);
        assert_eq!(got[0], [7, 0, 0]);
    }

    #[test]
    fn unbreakable_or_unloaded_origin_plans_nothing() {
        let w = world(&[]);
        assert!(run(&w, Shape::LargeTunnel, [0, 0, 0], Face::Up, 64).is_empty());
    }

    #[test]
    fn small_tunnel_goes_into_the_block() {
        let blocks: Vec<_> = (0..10).map(|y| ([0, -y, 0], 1)).collect();
        let w = world(&blocks);
        let got = run(&w, Shape::SmallTunnel, [0, 0, 0], Face::Up, 4);
        assert_eq!(got, vec![[0, 0, 0], [0, -1, 0], [0, -2, 0], [0, -3, 0]]);
    }

    #[test]
    fn large_tunnel_is_3x3_per_layer() {
        let mut blocks = Vec::new();
        for z in 0..4 {
            for x in -1..=1 {
                for y in -1..=1 {
                    blocks.push(([x, y, z], 3));
                }
            }
        }
        let w = world(&blocks);
        // Hit the north face (normal -z): tunnel goes +z.
        let got = run(&w, Shape::LargeTunnel, [0, 0, 0], Face::North, 18);
        assert_eq!(got.len(), 18);
        assert_eq!(got[0], [0, 0, 0]);
        assert!(got.iter().all(|p| p[2] <= 1));
    }

    #[test]
    fn squares_are_flat_and_sized() {
        let mut blocks = Vec::new();
        for x in -3..=3 {
            for z in -3..=3 {
                blocks.push(([x, 0, z], 1));
                blocks.push(([x, -1, z], 1));
            }
        }
        let w = world(&blocks);
        let small = run(&w, Shape::SmallSquare, [0, 0, 0], Face::Up, 64);
        let large = run(&w, Shape::LargeSquare, [0, 0, 0], Face::Up, 64);
        assert_eq!((small.len(), large.len()), (9, 25));
        assert!(small.iter().chain(&large).all(|p| p[1] == 0));
    }

    #[test]
    fn skips_gaps_but_keeps_the_rest() {
        let w = world(&[([0, 0, 0], 1), ([1, 0, 0], 1)]);
        let got = run(&w, Shape::SmallSquare, [0, 0, 0], Face::Up, 64);
        assert_eq!(got.len(), 2);
    }

    #[test]
    fn coordinate_overflow_does_not_panic() {
        let w = world(&[([i32::MAX, 0, 0], 1)]);
        let got = run(&w, Shape::Shapeless, [i32::MAX, 0, 0], Face::East, 64);
        assert_eq!(got, vec![[i32::MAX, 0, 0]]);
        let got = run(&w, Shape::LargeTunnel, [i32::MAX, 0, 0], Face::West, 64);
        assert_eq!(got.first(), Some(&[i32::MAX, 0, 0]));
    }

    #[test]
    fn shape_names_round_trip_and_cycle() {
        for s in Shape::ALL {
            assert_eq!(Shape::parse(s.name()), Some(s));
        }
        assert_eq!(Shape::LargeSquare.next(), Shape::Shapeless);
    }
}
