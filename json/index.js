const all = require('one-piece-card-game-json-all');
const enCards = require('./en/cards.json');
const enFilters = require('./en/filters.json');
const jp = require('one-piece-card-game-json-jp');

module.exports = {
  all,
  en: {
    cards: enCards,
    filters: enFilters
  },
  jp
};
