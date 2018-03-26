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

mod tile {
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
}

impl Policy {
    pub fn new() -> Self {
        let (entity_store, _wit) = EntityStore::new(Size::new(8, 8));
        Self {
            entity_store,
        }
    }
    pub fn make_player<'w>(&mut self, wit: &'w EntityWit<'w>, coord: Coord) -> EntityId<'w> {
        let id = self.entity_store.allocate_entity_id(wit);

        self.entity_store.insert_coord(id, coord);
        self.entity_store.insert_player(id);
        self.entity_store.insert_tile(id, tile::TileInfo { typ: tile::TileType::Player, depth: 3 });

        id
    }
    /*
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
    }*/
    pub fn render_iter<'a, 'w>(&'a self, wit: &'w EntityWit<'w>) -> RenderIter<'a, 'w> {
        RenderIter {
            iter: self.entity_store.iter_tile(wit),
            entity_store: &self.entity_store,
        }
    }
}

pub struct RenderIter<'a, 'w> {
    iter: ComponentIterTile<'a, 'w>,
    entity_store: &'a EntityStore,
}

pub enum Input {
}
pub struct GameState {
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
    //                '@' => { policy.make_player(coord); }
      //              '#' => { policy.make_wall(coord); }
      //              '.' => { policy.make_floor(coord); }
                    _ => panic!(),
                }
            }
        }

        Self {
        }
    }
    pub fn from_save_state(save_state: SaveState) -> Self {
        Self {

        }
    }
    pub fn save(&self, rng_seed: usize) -> SaveState {
        SaveState {

        }
    }
    pub fn tick<I>(&mut self, inputs: I, period: Duration) -> Option<ExternalEvent>
        where
        I: IntoIterator<Item = Input>,
    {
        None
    }
}

#[derive(Serialize, Deserialize)]
pub struct SaveState {

}
