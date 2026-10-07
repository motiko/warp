# password.ttf

`password.ttf` is a 1136-byte single-glyph TrueType font ("PasswordCircle") used to mask
password input: besides `.notdef` it contains one glyph, `circle`, and its cmap (format 13,
many-to-one) maps every code point to that glyph, so any text rendered with this font
appears as a row of identical circles.

## Origin and license

The font was created in-house for the Warp terminal and entered this repository with the
commit "Initial public release of Warp." (0dbd3d567) of https://github.com/warpdotdev/warp.
It credits no third party and is covered by the repository license, AGPL-3.0 (see the root
`LICENSE` file). The Alefbet fork redistributes it under the same terms.

## Name table

Both the Macintosh (platform 1) and Windows (platform 3, en-US) records carry the same
values:

| ID | Field               | Value                         |
| -- | ------------------- | ----------------------------- |
| 1  | Family name         | `PasswordCircle`              |
| 2  | Subfamily name      | `Regular`                     |
| 3  | Unique identifier   | `PasswordCircle-Regular-1.0`  |
| 4  | Full name           | `PasswordCircle Regular`      |
| 5  | Version             | `1.0`                         |
| 6  | PostScript name     | `PasswordCircle-Regular`      |

No copyright (ID 0), trademark (ID 7), manufacturer, designer, or license (IDs 13/14)
records are present. Tables: `OS/2`, `cmap`, `glyf`, `head`, `hhea`, `hmtx`, `loca`,
`maxp`, `name`, `post`. Glyph order: `.notdef`, `circle`.
