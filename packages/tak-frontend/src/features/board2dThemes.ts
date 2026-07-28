import z, { object } from 'zod';
import assassin from '../assets/board-2d/assassin.json';
import beach from '../assets/board-2d/beach.json';
import bubblegum from '../assets/board-2d/bubblegum.json';
import classic from '../assets/board-2d/classic.json';
import discord from '../assets/board-2d/discord.json';
import frost from '../assets/board-2d/frost.json';
import galaxy from '../assets/board-2d/galaxy.json';
import ignis from '../assets/board-2d/ignis.json';
import jungle from '../assets/board-2d/jungle.json';
import mushroom from '../assets/board-2d/mushroom.json';
import neon from '../assets/board-2d/neon.json';
import papyrus from '../assets/board-2d/papyrus.json';
import sakura from '../assets/board-2d/sakura.json';
import space from '../assets/board-2d/space.json';
import steampunk from '../assets/board-2d/steampunk.json';

const pieceColorSchema = z.object({
  background: z.string(),
  border: z.string(),
  text: z.string().optional(),
  capstoneOverride: z
    .object({
      background: z.string(),
      border: z.string(),
    })
    .optional(),
});

export const themeSchema = z
  .object({
    name: z.string(),
    background: z.string(),
    text: z.string(),
    board1: z.string(),
    board2: z.string(),
    tileSpecial: z
      .object({
        color: z.string(),
        border: z.string(),
        borderColor: z.string(),
        rounded: z.string().default('0'),
        size: z.string(),
        transform: z.string().optional(),
        hideBackground: z.boolean().default(false),
      })
      .strict()
      .optional(),
    highlight: z.string(),
    hover: z.string(),
    piece1: pieceColorSchema,
    piece2: pieceColorSchema,
    board: object({
      spacing: z.string(),
      rounded: z.string(),
      tiling: z.enum(['checkerboard', 'rings', 'linear', 'random']),
    }).strict(),
    pieces: object({
      rounded: z.number(),
      border: z.string(),
      shadow: z
        .object({
          opacity: z.number().min(0).max(1),
          blur: z.string(),
          offsetY: z.number(),
          color: z.string().optional(),
        })
        .strict()
        .optional(),
    }).strict(),
  })
  .strict();

export type ThemeParams = z.infer<typeof themeSchema>;

export const defaultTheme = themeSchema.parse(classic);

export const board2dThemeIds = [
  'classic',
  'jungle',
  'ignis',
  'neon',
  'discord',
  'beach',
  'assassin',
  'space',
  'sakura',
  'steampunk',
  'bubblegum',
  'frost',
  'papyrus',
  'galaxy',
  'mushroom',
] as const;

export type Board2dThemeId = (typeof board2dThemeIds)[number];

export const board2dThemes: Record<Board2dThemeId, ThemeParams> = {
  classic: defaultTheme,
  jungle: themeSchema.parse(jungle),
  ignis: themeSchema.parse(ignis),
  neon: themeSchema.parse(neon),
  discord: themeSchema.parse(discord),
  beach: themeSchema.parse(beach),
  assassin: themeSchema.parse(assassin),
  space: themeSchema.parse(space),
  sakura: themeSchema.parse(sakura),
  steampunk: themeSchema.parse(steampunk),
  bubblegum: themeSchema.parse(bubblegum),
  frost: themeSchema.parse(frost),
  papyrus: themeSchema.parse(papyrus),
  galaxy: themeSchema.parse(galaxy),
  mushroom: themeSchema.parse(mushroom),
};
