import type { Brand } from './types';
import { vulosMeta } from './vulos.meta';
import markSvg from 'virtual:brand-mark/vulos';

export const brand: Brand = { ...vulosMeta, markSvg };
