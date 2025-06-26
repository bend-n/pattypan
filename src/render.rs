use std::iter::zip;
use std::sync::LazyLock;

use atools::Join;
use fimg::BlendingOverlayAt;
use swash::scale::{Render, ScaleContext, Source};
use swash::shape::cluster::Glyph;
use swash::shape::{ShapeContext, ShaperBuilder};
use swash::text::cluster::{CharCluster, CharInfo, Parser, Token};
use swash::text::{Codepoint, Language, Script};
use swash::zeno::Format;
use swash::{FontRef, Instance};

use crate::colors;
use crate::term::{BOLD, ITALIC};

#[implicit_fn::implicit_fn]
pub fn render(
    x: &mut super::Terminal,
    (w, h): (usize, usize),
    ppem: f32,
) -> Image<Box<[u8]>, 3> {
    let m = FONT.metrics(&[]);
    let sz = ppem * (m.max_width / m.units_per_em as f32);
    let mut i = Image::build(w as _, h as _).fill(colors::BACKGROUND);
    if w < 60 || h < 60 {
        return i;
    }
    let c = x.cells.c() as usize;
    let r = x.cells.r() as usize;
    let vo = x.view_o.unwrap_or(x.cells.row);
    for (col, k) in x.cells.cells[vo * c..vo * c + r * c]
        .chunks_exact(c as _)
        .zip(1..)
    {
        for (&(mut cell), j) in zip(col, 0..) {
            if cell.style.flags & crate::term::INVERT != 0 {
                std::mem::swap(&mut cell.style.bg, &mut cell.style.color);
            }
            if cell.style.bg != colors::BACKGROUND {
                let cell = Image::<_, 4>::build(
                    sz.ceil() as u32,
                    (ppem * 1.25).ceil() as u32,
                )
                .fill(cell.style.bg.join(255));
                unsafe {
                    i.as_mut().overlay_at(
                        &cell,
                        4 + (j as f32 * sz) as u32,
                        (k as f32 * (ppem * 1.25)) as u32 - 20,
                    )
                };
            }
        }
    }
    let mut characters: Vec<Glyph> = vec![Glyph::default(); c];

    for (col, k) in x.cells.cells[vo * c..vo * c + r * c]
        .chunks_exact(c as _)
        .zip(1..)
    {
        let tokenized = col.iter().enumerate().map(|(i, cell)| {
            let ch = cell.letter.unwrap_or(' ');
            (
                Token {
                    ch,
                    offset: i as u32,
                    len: ch.len_utf8() as u8,
                    info: ch.properties().into(),
                    data: i as u32,
                },
                cell.style.flags,
            )
        });

        macro_rules! input {
            ($rule:expr, $font:ident) => {
                let mut scx = ShapeContext::new();
                let mut shaper = scx
                    .builder(*$font)
                    .size(ppem)
                    .script(Script::Latin)
                    .features([
                        ("ss19", 1),
                        ("ss01", 1),
                        ("ss20", 1),
                        ("liga", 1),
                        ("rlig", 1),
                    ])
                    .build();

                let mut cluster = CharCluster::new();

                let mut parser = Parser::new(
                    Script::Latin,
                    tokenized.clone().filter($rule).map(|x| x.0),
                );
                while parser.next(&mut cluster) {
                    cluster.map(|ch| $font.charmap().map(ch));
                    shaper.add_cluster(&cluster);
                }
                shaper.shape_with(|x| {
                    x.glyphs.into_iter().for_each(|x| {
                        characters[x.data as usize] = *x;
                    })
                });
            };
        }
        input!(|x| x.1 & ITALIC == 0, FONT);
        input!(|x| x.1 & ITALIC != 0, IFONT); // bifont is an instance of ifont

        for (&(mut cell), glyph) in
            characters.iter().map(|x| (&col[x.data as usize], x))
        {
            let j = glyph.data as usize;
            if cell.style.flags & crate::term::INVERT != 0 {
                std::mem::swap(&mut cell.style.bg, &mut cell.style.color);
            }
            let mut color = cell.style.color;
            if (cell.style.flags & crate::term::DIM) != 0 {
                color = color.map(|x| x / 2);
            }
            if (cell.style.flags & crate::term::UNDERLINE) != 0 {
                unsafe {
                    i.as_mut().overlay_at(
                        &Image::<_, 4>::build(sz.ceil() as u32, 2)
                            .fill(color.join(255)),
                        4 + (j as f32 * sz) as u32,
                        (k as f32 * (ppem * 1.25)) as u32 + 5,
                    )
                };
            }
            if (cell.style.flags & crate::term::STRIKETHROUGH) != 0 {
                unsafe {
                    i.as_mut().overlay_at(
                        &Image::<_, 4>::build(sz.ceil() as u32, 2)
                            .fill(color.join(255)),
                        4 + (j as f32 * sz) as u32,
                        (k as f32 * (ppem * 1.25)) as u32 - 5,
                    )
                };
            }
            if let Some(_) = cell.letter {
                let f = if (cell.style.flags & crate::term::ITALIC) != 0 {
                    *IFONT
                } else {
                    *FONT
                };
                let id = glyph.id;
                let mut scbd = ScaleContext::new();
                let mut scbd = scbd.builder(f);
                scbd = scbd.size(ppem);
                if (cell.style.flags & crate::term::BOLD) != 0 {
                    scbd = scbd.variations(
                        if (cell.style.flags & crate::term::ITALIC) != 0 {
                            *BIFONT
                        } else {
                            *BFONT
                        }
                        .values()
                        .zip(f.variations())
                        .map(|x| (x.1.tag(), x.0)),
                    );
                }
                let x = Render::new(&[Source::Outline])
                    .format(Format::Alpha)
                    .render(&mut scbd.build(), id)
                    .unwrap();
                unsafe {
                    if x.placement.width == 0 {
                        continue;
                    }
                    let item = Image::<_, 1>::build(
                        x.placement.width,
                        x.placement.height,
                    )
                    .buf_unchecked(x.data);
                    i.as_mut().blend_alpha_and_color_at(
                        &item.as_ref(),
                        color,
                        4 + ((j as f32 * sz + glyph.x)
                            + x.placement.left as f32)
                            .round() as u32,
                        ((k as f32 * (ppem * 1.25)) as u32)
                            .saturating_sub(x.placement.top as u32),
                    );
                }
            }
        }
    }

    if x.view_o == Some(x.cells.row) || x.view_o.is_none() {
        let cell = Image::<_, 4>::build(3, (ppem * 1.25).ceil() as u32)
            .fill([0xFF, 0xCC, 0x66, 255]);
        unsafe {
            i.as_mut().overlay_at(
                &cell,
                4 + ((x.cursor.0 - 1) as f32 * sz) as u32,
                (x.cursor.1 as f32 * (ppem * 1.25)) as u32 - 20,
            )
        };
    }
    i
}

pub fn dims(font: &FontRef, ppem: f32) -> (f32, f32) {
    let m = font.metrics(&[]);
    (ppem * (m.max_width / m.units_per_em as f32), ppem * 1.25)
}

pub static FONT: LazyLock<FontRef<'static>> = LazyLock::new(|| {
    FontRef::from_index(&include_bytes!("../CascadiaCodeNF.ttf")[..], 0)
        .unwrap()
});

pub static IFONT: LazyLock<FontRef<'static>> = LazyLock::new(|| {
    FontRef::from_index(
        &include_bytes!("../CascadiaCodeNFItalic.ttf")[..],
        0,
    )
    .unwrap()
});

pub static BIFONT: LazyLock<Instance<'static>> = LazyLock::new(|| {
    IFONT.instances().find_by_name("Bold Italic").unwrap()
});

pub static BFONT: LazyLock<Instance<'static>> =
    LazyLock::new(|| FONT.instances().find_by_name("Bold").unwrap());

use fimg::{Image, OverlayAt};
// let x = b"echo -e \"\x1b(0lqqqk\nx   \x1b(Bx\nmqqqj";
// let x = String::from_utf8_lossy(&x);
// println!("{}", x);
#[test]
fn t() {
    let f = *FONT;
    f.features()
        .for_each(|f| drop(dbg!(f.name().unwrap().to_string())));
    // let f = FontRef::from_index(&include_bytes!("../fira.ttf")[..], 0)
    //     .unwrap();

    dbg!(f.attributes());
    // let m = f.metrics(&[f.charmap().map('行') as _]);

    let ppem = 30.0;
    let d = dims(&FONT, ppem);
    let sz = d.0;
    let mut scx = ShapeContext::new();
    let mut shaper = scx
        .builder(f)
        .size(ppem)
        .script(Script::Latin)
        .features([
            ("ss19", 1),
            ("ss01", 1),
            ("ss20", 1),
            ("liga", 1),
            ("rlig", 1),
        ])
        .build();

    let mut cluster = CharCluster::new();
    let mut parser = Parser::new(
        Script::Latin,
        "!=".char_indices().map(|(i, ch)| Token {
            ch,
            offset: i as u32,
            len: ch.len_utf8() as u8,
            info: ch.properties().into(),
            data: 0,
        }),
    );

    while parser.next(&mut cluster) {
        cluster.map(|ch| f.charmap().map(ch));
        shaper.add_cluster(&cluster);
    }
    let mut characters: Vec<Glyph> = vec![];
    shaper.shape_with(|x| {
        dbg!(x.is_ligature());
        dbg!(x);
        characters.extend(x.glyphs)
    });
    dbg!(f.charmap().map('!'));
    let mut grid = Image::<_, 4>::alloc(2000, 500);
    grid.chunked_mut().for_each(|x| *x = [0, 0, 0, 255]);
    for (letter, i) in characters.iter().zip(0..) {
        // grid.as_ref().show();
        // let id = f.charmap().map(letter);
        // let id = 1940;
        let id = letter.id;
        let x = Render::new(&[Source::Outline])
            .format(Format::Alpha)
            .render(
                &mut ScaleContext::new().builder(f).size(ppem).build(),
                id,
            )
            .unwrap();
        dbg!(x.placement.width);
        dbg!(letter.x);

        unsafe {
            if x.placement.width == 0 {
                continue;
            }
            grid.as_mut().overlay_blended_at(
                &Image::<Box<[u8]>, 4>::from(
                    Image::<_, 1>::build(
                        x.placement.width,
                        x.placement.height,
                    )
                    .buf(&*x.data)
                    .show(),
                )
                .as_ref(),
                ((i as f32 * sz).round() as i32 + x.placement.left) as _,
                ppem as u32 - x.placement.top as u32,
            );
        }
    }
    for (letter, i) in "#".repeat(21 * 2).chars().zip(0..) {
        // grid.as_ref().show();
        let id = FONT.charmap().map(letter);
        let x = Render::new(&[Source::Outline])
            .format(Format::Alpha)
            .render(
                &mut ScaleContext::new().builder(*FONT).size(ppem).build(),
                id,
            )
            .unwrap();
        unsafe {
            if x.placement.width == 0 {
                continue;
            }
            grid.as_mut().overlay_at(
                &Image::<Box<[u8]>, 4>::from(
                    Image::<_, 1>::build(
                        x.placement.width,
                        x.placement.height,
                    )
                    .buf(&*x.data),
                )
                .as_ref(),
                ((i as f32 * sz) as i32 + x.placement.left) as _,
                30 + ppem as u32 - x.placement.top as u32,
            );
        }
    }
    grid.show();
}
