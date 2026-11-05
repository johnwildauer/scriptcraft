# Material Symbols Rounded font asset

- Upstream project: [Google Material Design Icons](https://github.com/google/material-design-icons)
- Upstream commit: `10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7`
- Source file: [`MaterialSymbolsRounded[FILL,GRAD,opsz,wght].ttf`](https://github.com/google/material-design-icons/blob/10ecc14790612ca8fd164e3f4ffe9c16fc50d5b7/variablefont/MaterialSymbolsRounded%5BFILL%2CGRAD%2Copsz%2Cwght%5D.ttf)
- Local file: `fonts/MaterialSymbols/MaterialSymbolsRounded[FILL,GRAD,opsz,wght].ttf`
- SHA-256: `7e74e36533ccee12c2b81e75bfcf3ccf123d3bb7e7e46d65d94445f23db0c97c`
- License: Apache License 2.0; the upstream license is copied to [`LICENSE`](LICENSE).
- Variable axes: `FILL` 0–1, `GRAD` -50–200, `opsz` 20–48, and `wght` 100–700, as published by the upstream font.
- The TTF is bundled unmodified. ScriptCraft selects outlined (`FILL=0`) and filled (`FILL=1`) faces at `GRAD=0`, `opsz=24`, and `wght=200` by default.
- The app registers all seven standard weights (100–700) at the default optical size and grade. Other optical-size/grade combinations are registered only when explicitly requested by a toolbar icon's typed override; those combinations use separate named faces for each selected fill style and weight.
- The currently exposed typed optical-size choices are 20, 24, and 48; grade choices are -50, 0, and 200. All lie within the source font's published axis ranges above.

The app registers these faces under the existing `egui_material_icons` family names, preserving typed icon codepoints and existing rendering call sites. This record identifies the bundled asset; it makes no runtime or visual validation claim.
