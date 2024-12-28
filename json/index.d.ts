declare module 'one-piece-card-game-json-all' {
  const all: any;
  export default all;
}

declare module 'one-piece-card-game-json-en' {
  const en: any;
  export default en;
}

declare module 'one-piece-card-game-json-jp' {
  const jp: any;
  export default jp;
}

declare module 'one-piece-card-game-json' {
  const all: any;
  const en: {
    cards: any;
    filters: any;
  };
  const jp: any;

  export { all, en, jp };
}
