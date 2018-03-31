extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate entity_store_helper;

use std::time::Duration;
use entity_store_helper::grid_2d::{Size, Coord};

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

pub struct Policy {
    entity_store: EntityStore,
    wit: EntityWit,
}

impl Policy {
    pub fn new() -> Self {
        let (entity_store, wit) = EntityStore::new(Size::new(8, 8));
        Self {
            entity_store,
            wit,
        }
    }
    pub fn make_player(&mut self, coord: Coord) -> EntityId {
        let id = self.entity_store.allocate_entity_id(&self.wit);

        self.entity_store.insert_coord(id, coord);
        self.entity_store.insert_player(id);
        self.entity_store.insert_tile(id, tile::TileInfo { typ: tile::TileType::Player, depth: 3 });

        id
    }
    pub fn make_wall(&mut self, coord: Coord) -> EntityId {
        let id = self.entity_store.allocate_entity_id(&self.wit);

        self.entity_store.insert_coord(id, coord);
        self.entity_store.insert_solid(id);
        self.entity_store.insert_tile(id, tile::TileInfo { typ: tile::TileType::Wall, depth: 2 });

        id
    }
    pub fn make_floor(&mut self, coord: Coord) -> EntityId {
        let id = self.entity_store.allocate_entity_id(&self.wit);

        self.entity_store.insert_coord(id, coord);
        self.entity_store.insert_tile(id, tile::TileInfo { typ: tile::TileType::Floor, depth: 1 });

        id
    }
    pub fn render_iter(&self) -> RenderIter {
        RenderIter {
            iter: self.entity_store.iter_tile(&self.wit),
            entity_store: &self.entity_store,
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
                    '@' => { policy.make_player(coord); }
                    '#' => { policy.make_wall(coord); }
                    '.' => { policy.make_floor(coord); }
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
                entity_store: save_state.entity_store.clone(),
                wit: save_state.wit,
            },
        }
    }
    pub fn save(&self, rng_seed: usize) -> SaveState {
        SaveState {
            entity_store: self.policy.entity_store.clone(),
            wit: self.policy.wit,
        }
    }
    pub fn tick<I>(&mut self, inputs: I, period: Duration) -> Option<ExternalEvent>
        where
        I: IntoIterator<Item = Input>,
    {
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
