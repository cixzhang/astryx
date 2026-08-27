Frames for facebook/astryx#5601 (Night Watch component audit of core/Carousel, 2026-08-27).

`before/` is origin/main's `Carousel.tsx` built in the same worktree with the same
pipeline as `after/`, so the pair differs only by the diff. Each PNG has a
`.sensors.json` receipt beside it recording the build SHA, story id, theme,
color mode, direction, viewport, semantic state, geometry and settled render.

25 of 25 comparable frames are byte-identical. `after/Carousel__default__after-edge-press__light.png`
has no `before/` twin: on main that capture cannot be taken, because the sensor
reads focus as BODY instead of the scroll container.
