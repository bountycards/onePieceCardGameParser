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
  import all from 'one-piece-card-game-json-all';
  import en from 'one-piece-card-game-json-en';
  import jp from 'one-piece-card-game-json-jp';

  export { all, en, jp };
}
