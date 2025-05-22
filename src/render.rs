use std::sync::LazyLock;

use atools::Join;
use fimg::BlendingOverlayAt;
use swash::FontRef;
use swash::scale::{Render, ScaleContext, Source};
use swash::zeno::Format;

use crate::colors;

#[implicit_fn::implicit_fn]
pub fn render(
    x: &super::Terminal,
    (w, h): (usize, usize),
    ppem: f32,
) -> Image<Box<[u8]>, 3> {
    let m = FONT.metrics(&[]);
    let sz = ppem * (m.max_width / m.units_per_em as f32);
    let mut i = Image::build(w as _, h as _).fill(colors::BACKGROUND);
    for (col, k) in x.cells.chunks_exact(x.size.0 as _).zip(0..).skip(1) {
        for (cell, j) in
            col.iter().skip(2).zip(0..).filter(_.0.letter.is_some())
        {
            let id = FONT.charmap().map(cell.letter.unwrap());

            let x = Render::new(&[Source::Outline])
                .format(Format::Alpha)
                .render(
                    &mut ScaleContext::new()
                        .builder(*FONT)
                        .size(ppem)
                        .build(),
                    id,
                )
                .unwrap();
            unsafe {
                if x.placement.width == 0 {
                    continue;
                }
                let item = Image::<_, 4>::build(
                    x.placement.width,
                    x.placement.height,
                )
                .buf(
                    x.data
                        .iter()
                        .flat_map(|&x| cell.style.color.join(x))
                        .collect::<Vec<u8>>(),
                );
                if cell.style.bg != colors::BACKGROUND {
                    let cell = Image::<_, 4>::build(
                        sz.ceil() as u32,
                        (ppem * 1.25).ceil() as u32,
                    )
                    .fill(cell.style.bg.join(255));
                    i.as_mut().overlay_at(
                        &cell,
                        4 + (j as f32 * sz) as u32,
                        (k as f32 * (ppem * 1.25)) as u32 - 20,
                    );
                }

                i.as_mut().overlay_blended_at(
                    &item.as_ref(),
                    // &Image::<Box<[u8]>, 4>::from(
                    //     Image::<_, 1>::build(
                    //         x.placement.width,
                    //         x.placement.height,
                    //     )
                    //     .buf(&*x.data),
                    // )
                    // .as_ref(),
                    4 + ((j as f32 * sz) + x.placement.left as f32) as u32,
                    ((k as f32 * (ppem * 1.25)) as u32)
                        .saturating_sub(x.placement.top as u32),
                );
            }
        }
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

use fimg::{BlendingOverlay, Image, OverlayAt};
// let x = b"echo -e \"\x1b(0lqqqk\nx   \x1b(Bx\nmqqqj";
// let x = String::from_utf8_lossy(&x);
// println!("{}", x);
#[test]
fn t() {
    let f =
        FontRef::from_index(&include_bytes!("../cjk.ttc")[..], 0).unwrap();

    dbg!(f.attributes());
    let m = f.metrics(&[f.charmap().map('行') as _]);

    let ppem = 30.0;
    let d = dims(&FONT, ppem);
    let sz = d.0;
    let mut grid = Image::<_, 4>::alloc(2000, 500);
    unsafe { grid.chunked_mut().for_each(|x| *x = [0, 0, 0, 255]) };
    for (letter, i) in "素早い茶色のキツネは怠け者の犬を飛び越えた"
        .chars()
        .zip(0..)
    {
        // grid.as_ref().show();
        let id = f.charmap().map(letter);
        let x = Render::new(&[Source::Outline])
            .format(Format::Alpha)
            .render(
                &mut ScaleContext::new().builder(f).size(ppem).build(),
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
                ((i as f32 * sz * 2.0) as i32 + x.placement.left) as _,
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
