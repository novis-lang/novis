/** A part's rect in world units: `[x, y, w, h]`. */
export type RectProp = [number, number, number, number]

/** Where a part sits: in the 16:9 `wide` layout and, for phones, the `tall` one. */
export interface Place {
  wide: RectProp
  tall?: RectProp
}

/** The data attributes the stage reads a part's rects from. */
export function placeAttrs(place: Place): Record<string, string> {
  const out: Record<string, string> = { 'data-wide': place.wide.join(',') }
  if (place.tall) out['data-tall'] = place.tall.join(',')
  return out
}
