extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate entity_store_helper;
extern crate direction;

use std::time::Duration;
use entity_store_helper::grid_2d::{Size, Coord};
use direction::CardinalDirection;

mod entity_store {
    include_entity_store!("entity_store.rs");
}

use entity_store::*;

pub mod tile {
    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub enum TileType {
        Floor,
        Wall,
        Player,
    }
    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct TileInfo {
        pub typ: TileType,
        pub depth: i8,
    }
}

pub struct LowLevelPolicy {
    entity_store: EntityStore,
}

impl LowLevelPolicy {
    pub fn make_player(&mut self, id: EntityId, coord: Coord) {
        self.entity_store.insert_coord(id, coord);
        self.entity_store.insert_player(id);
        self.entity_store.insert_tile(id, tile::TileInfo { typ: tile::TileType::Player, depth: 3 });
    }
    pub fn make_wall(&mut self, id: EntityId, coord: Coord) {
        self.entity_store.insert_coord(id, coord);
        self.entity_store.insert_solid(id);
        self.entity_store.insert_tile(id, tile::TileInfo { typ: tile::TileType::Wall, depth: 2 });
    }
    pub fn make_floor(&mut self, id: EntityId, coord: Coord) {
        self.entity_store.insert_coord(id, coord);
        self.entity_store.insert_tile(id, tile::TileInfo { typ: tile::TileType::Floor, depth: 1 });
    }
    pub fn move_collider_check(&self, id: EntityId, direction: CardinalDirection) -> Option<Coord> {
        if let Some(current_coord) = self.entity_store.get_coord(id) {
            let new_coord = current_coord + direction.coord();
            if let Some(destination_cell) = self.entity_store.spatial_hash_get(new_coord) {
                if destination_cell.solid_count == 0 {
                    return Some(new_coord);
                }
            }
        }
        None
    }
    pub fn move_collider(&mut self, id: EntityId, direction: CardinalDirection) {
        if let Some(new_coord) = self.move_collider_check(id, direction) {
            self.entity_store.insert_coord(id, new_coord);
        }
    }
}

pub struct Policy {
    low_level: LowLevelPolicy,
    wit: EntityWit,
}

impl Policy {
    pub fn new() -> Self {
        let (entity_store, wit) = EntityStore::new(Size::new(8, 8));
        Self {
            low_level: LowLevelPolicy {
                entity_store,
            },
            wit,
        }
    }
    pub fn move_player(&mut self, direction: CardinalDirection) {
        let player_id = self.low_level.entity_store.any_player(&self.wit).unwrap();
        self.low_level.move_collider(player_id, direction);
    }
    pub fn make_player(&mut self, coord: Coord) {
        let id = self.low_level.entity_store.allocate_entity_id(&self.wit);
        self.low_level.make_player(id, coord);
    }
    pub fn make_wall(&mut self, coord: Coord) {
        let id = self.low_level.entity_store.allocate_entity_id(&self.wit);
        self.low_level.make_wall(id, coord);
    }
    pub fn make_floor(&mut self, coord: Coord) {
        let id = self.low_level.entity_store.allocate_entity_id(&self.wit);
        self.low_level.make_floor(id, coord);
    }
    pub fn render_iter(&self) -> RenderIter {
        RenderIter {
            iter: self.low_level.entity_store.iter_tile(&self.wit),
            entity_store: &self.low_level.entity_store,
        }
    }
}

pub struct RenderIter<'a> {
    iter: ComponentIterTile<'a, 'a>,
    entity_store: &'a EntityStore,
}

impl<'a> Iterator for RenderIter<'a> {
    type Item = (&'a Coord, &'a tile::TileInfo);
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((id, tile)) = self.iter.next() {
            if let Some(coord) = self.entity_store.get_coord(id) {
                return Some((coord, tile));
            }
        }
        None
    }
}

pub enum Input {
    Move(CardinalDirection),
}
pub struct GameState {
    policy: Policy,
}
pub enum ExternalEvent {
    GameOver,
}

impl GameState {
    pub fn from_rng_seed(rng_seed: usize) -> Self {

        let strings = vec![
            "########",
            "#......#",
            "#......#",
            "#......#",
            "#..#####",
            "#......#",
            "#..@...#",
            "########",
        ];

        let mut policy = Policy::new();

        for (y, line) in strings.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                let coord = Coord::new(x as i32, y as i32);
                match ch {
                    '@' => {
                        policy.make_player(coord);
                        policy.make_floor(coord);
                    }
                    '#' => policy.make_wall(coord),
                    '.' => policy.make_floor(coord),
                    _ => panic!(),
                }
            }
        }

        Self {
            policy,
        }
    }
    pub fn from_save_state(save_state: SaveState) -> Self {
        Self {
            policy: Policy {
                low_level: LowLevelPolicy {
                    entity_store: save_state.entity_store.clone(),
                },
                wit: save_state.wit,
            },
        }
    }
    pub fn save(&self, rng_seed: usize) -> SaveState {
        SaveState {
            entity_store: self.policy.low_level.entity_store.clone(),
            wit: self.policy.wit,
        }
    }
    pub fn tick<I>(&mut self, inputs: I, period: Duration) -> Option<ExternalEvent>
        where
        I: IntoIterator<Item = Input>,
    {
        for input in inputs {
            match input {
                Input::Move(direction) => {
                    self.policy.move_player(direction);
                }
            }
        }

        None
    }
    pub fn render_iter(&self) -> RenderIter {
        self.policy.render_iter()
    }
}

#[derive(Serialize, Deserialize)]
pub struct SaveState {
    entity_store: EntityStore,
    wit: EntityWit,
}
