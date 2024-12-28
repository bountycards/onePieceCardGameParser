export interface Card {
  card_name: string;
  card_number: string;
  rarity: string;
  is_alternate_art: boolean;
  card_type: string;
  image_url: string;
  life: string;
  cost: string;
  attributes: string[];
  power: string;
  counter: string;
  colors: string[];
  types: string[];
  effects: string;
  card_effects: string[];
  card_sets: string;
  image_name: string;
}

export interface Filters {
  colors: string[];
  rarities: string[];
  types: string[];
  sets: string[];
  categories: string[];
}

declare module 'one-piece-card-game-json' {
  export const all: Card[];
  export const jp: Card[];
  export const en: {
    cards: Card[];
    filters: Filters;
  };
}