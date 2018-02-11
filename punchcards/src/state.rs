use std::time::Duration;
use grid_2d::{Size, Coord};
use grid_search::SearchContext;
use entity_store::*;
use input::Input;
use policy;
use prototypes;
use card::*;
use card_state::*;
use tile::*;
use reaction::*;
use animation::*;
use rand::{StdRng, SeedableRng};
use append::Append;

const INITIAL_HAND_SIZE: usize = 4;

pub enum Meta {
    GameOver,
}

#[derive(Clone, Debug)]
pub struct GameState {
    entity_store: EntityStore,
    spatial_hash: SpatialHashTable,
    entity_components: EntityComponentTable,
    id_allocator: EntityIdAllocator,
    count: u64,
}

impl GameState {
    fn delete_entity<A: Append<EntityChange>>(&mut self, entity_id: EntityId, changes: &mut A) {
        for component in self.entity_components.components(entity_id) {
            changes.append(EntityChange::Remove(entity_id, component));
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum InputState {
    WaitingForCardSelection,
    WaitingForDirection(HandIndex, Card),
}

#[derive(Clone, Debug)]
pub struct State {
    game_state: GameState,
    player_id: EntityId,
    changes: Vec<EntityChange>,
    reactions: Vec<Reaction>,
    animations: Vec<Animation>,
    card_state: CardState,
    input_state: InputState,
    search_context: SearchContext<u32>,
    rng: StdRng,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveState {
    changes: Vec<EntityChange>,
    id_allocator: EntityIdAllocator,
    count: u64,
    player_id: EntityId,
    card_state: CardState,
    animations: Vec<Animation>,
    input_state: InputState,
    next_rng_seed: usize,
    size: Size,
}

impl State {
    pub fn new(rng_seed: usize) -> Self {

        let mut rng = StdRng::from_seed(&[rng_seed]);

        let strings = vec![
            "##########",
            "#..m.....#",
            "#....#...#",
            "#.0@.#...#",
            "#....#...#",
            "#.####...#",
            "#........#",
            "#........#",
            "#........#",
            "##########",
        ];

        let size = Size::new(strings[0].len() as u32, strings.len() as u32);

        let mut entity_store = EntityStore::new();
        let mut spatial_hash = SpatialHashTable::new(size);
        let mut id_allocator = EntityIdAllocator::new();
        let mut changes = Vec::new();
        let animations = Vec::new();
        let mut player_id = None;

        for (y, line) in strings.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                let coord = Coord::new(x as i32, y as i32);
                match ch {
                    '#' => {
                        prototypes::wall(id_allocator.allocate(), coord, &mut changes);
                        prototypes::floor(id_allocator.allocate(), coord, &mut changes);
                    }
                    '.' => {
                        prototypes::floor(id_allocator.allocate(), coord, &mut changes);
                    }
                    'm' => {
                        prototypes::card(
                            id_allocator.allocate(),
                            coord,
                            Card::Move,
                            Tile::CardMove,
                            &mut changes,
                        );
                        prototypes::floor(id_allocator.allocate(), coord, &mut changes);
                    }
                    '0' => {
                        prototypes::target_dummy(id_allocator.allocate(), coord, &mut changes);
                        prototypes::floor(id_allocator.allocate(), coord, &mut changes);
                    }
                    '@' => {
                        let id = id_allocator.allocate();
                        player_id = Some(id);
                        prototypes::player(id, coord, &mut changes);
                        prototypes::floor(id_allocator.allocate(), coord, &mut changes);
                    }
                    _ => panic!("unexpected character"),
                }
            }
        }

        let player_id = player_id.expect("No player in level");

        let mut entity_components = EntityComponentTable::new();

        for change in changes.drain(..) {
            spatial_hash.update(&entity_store, &change, 0);
            entity_components.update(&change);
            entity_store.commit(change);
        }

        let card_state = CardState::new(
            vec![
                Card::Punch,
                Card::Punch,
                Card::Punch,
                Card::Punch,
                Card::Punch,
                Card::Move,
                Card::Move,
                Card::Move,
                Card::Move,
                Card::Move,
            ],
            INITIAL_HAND_SIZE,
            &mut rng,
        );

        Self {
            game_state: GameState {
                entity_store,
                spatial_hash,
                entity_components,
                id_allocator,
                count: 0,
            },
            input_state: InputState::WaitingForCardSelection,
            player_id,
            changes,
            animations,
            reactions: Vec::new(),
            card_state,
            search_context: SearchContext::new(size),
            rng,
        }
    }

    pub fn from_save_state(
        SaveState {
            mut changes,
            id_allocator,
            count,
            player_id,
            card_state,
            animations,
            input_state,
            next_rng_seed,
            size,
        }: SaveState,
    ) -> Self {
        let mut entity_store = EntityStore::new();
        let mut spatial_hash = SpatialHashTable::new(size);
        let mut entity_components = EntityComponentTable::new();

        for change in changes.drain(..) {
            spatial_hash.update(&entity_store, &change, 0);
            entity_components.update(&change);
            entity_store.commit(change);
        }

        Self {
            game_state: GameState {
                entity_store,
                spatial_hash,
                entity_components,
                id_allocator,
                count,
            },
            input_state,
            player_id,
            changes: Vec::new(),
            animations,
            reactions: Vec::new(),
            card_state,
            search_context: SearchContext::new(size),
            rng: StdRng::from_seed(&[next_rng_seed]),
        }
    }

    pub fn create_save_state(&self, next_rng_seed: usize) -> SaveState {
        let mut changes = Vec::with_capacity(1024);
        self.game_state.entity_store.clone_changes(&mut changes);
        SaveState {
            changes,
            id_allocator: self.game_state.id_allocator.clone(),
            count: self.game_state.count,
            player_id: self.player_id,
            card_state: self.card_state.clone(),
            animations: self.animations.clone(),
            input_state: self.input_state.clone(),
            next_rng_seed,
            size: Size::new(self.game_state.spatial_hash.width(), self.game_state.spatial_hash.height()),
        }
    }

    pub fn entity_store(&self) -> &EntityStore {
        &self.game_state.entity_store
    }
    pub fn spatial_hash(&self) -> &SpatialHashTable {
        &self.game_state.spatial_hash
    }
    pub fn card_state(&self) -> &CardState {
        &self.card_state
    }
    pub fn input_state(&self) -> &InputState {
        &self.input_state
    }

    pub fn tick<I>(&mut self, inputs: I, period: Duration) -> Option<Meta>
    where
        I: IntoIterator<Item = Input>,
    {
        let mut played_card = None;
        let mut input_state_change = None;

        if self.animations.is_empty() {
            for input in inputs {
                match input {
                    Input::SelectCard(index) => {
                        if let Some(card) = self.card_state.hand.get(index) {
                            input_state_change =
                                Some(InputState::WaitingForDirection(index, *card));
                        }
                    }
                    Input::Direction(direction) => {
                        if let InputState::WaitingForDirection(index, card) = self.input_state {
                            played_card = Some((index, card, direction));
                        }
                    }
                }
            }
        } else {
            for animation in self.animations.drain(..) {
                animation.step(period, &mut self.reactions);
            }
        }

        if let Some(input_state) = input_state_change {
            self.input_state = input_state;
        }

        if let Some((_, card, direction)) = played_card {
            card.play(
                self.player_id,
                &self.game_state.entity_store,
                direction,
                &mut self.game_state.id_allocator,
                &mut self.changes,
                &mut self.reactions,
            );
        }

        loop {
            for change in self.changes.drain(..) {
                if !policy::check(
                    &change,
                    &self.game_state.entity_store,
                    &self.game_state.spatial_hash,
                    &mut self.reactions,
                ) {
                    continue;
                }

                if let Some((index, card, _)) = played_card.take() {
                    let card_to_check = self.card_state.hand.remove_card(index);
                    assert_eq!(card, card_to_check);
                    self.card_state.fill_hand();
                    self.input_state = InputState::WaitingForCardSelection;
                }

                self.game_state.spatial_hash.update(
                    &self.game_state.entity_store,
                    &change,
                    self.game_state.count,
                );
                self.game_state.entity_components.update(&change);
                self.game_state.entity_store.commit(change);
                self.game_state.count += 1;
            }

            if self.reactions.is_empty() {
                if self.card_state.hand.is_empty() {
                    break Some(Meta::GameOver);
                } else {
                    break None;
                }
            } else {
                for reaction in self.reactions.drain(..) {
                    match reaction {
                        Reaction::TakeCard(entity_id, card) => {
                            self.card_state.deck.add_random(card, &mut self.rng);
                            self.game_state.delete_entity(entity_id, &mut self.changes);
                        }
                        Reaction::RemoveEntity(entity_id) => {
                            self.game_state.delete_entity(entity_id, &mut self.changes);
                        }
                        Reaction::StartAnimation(animation) => {
                            self.animations.push(animation);
                        }
                        Reaction::EntityChange(change) => {
                            self.changes.push(change);
                        }
                    }
                }
            }
        }
    }
}
