import {
  FM_BOOK_COVER,
  FM_BOOK_FILE,
  FM_PAGE_COVER,
  FM_READ_PERCENT,
  FM_READER_POSITION,
} from '../docs/frontmatter';

const CREATED = 'created: {{date:DD-MM-YYYY}} | ({{time:HH:mm}})';
const LINKS = ['***', '', 'Ссылки:', '- [[]]', ''];

export const noteStarterTemplate = [
  '---',
  CREATED,
  'tags: []',
  'author: ',
  'source: ',
  '---',
  '',
  '',
  '',
  '',
  ...LINKS,
].join('\n');

export const bookStarterTemplate = [
  '---',
  CREATED,
  'cover: true',
  'type: book',
  'author: ',
  'status: to-read',
  'pages: ',
  'tags: [книга, ]',
  'rating: 0',
  `${FM_BOOK_COVER}: `,
  `${FM_PAGE_COVER}: pattern:tunnel`,
  `${FM_BOOK_FILE}: `,
  `${FM_READER_POSITION}: `,
  `${FM_READ_PERCENT}: `,
  '---',
  '',
  '',
  '',
  ...LINKS,
].join('\n');
