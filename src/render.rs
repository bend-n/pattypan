use std::sync::LazyLock;

use swash::FontRef;
use swash::scale::{Render, ScaleContext, Source};
use swash::zeno::Format;

#[implicit_fn::implicit_fn]
pub fn render(
    x: &super::Terminal,
    (w, h): (usize, usize),
    ppem: f32,
) -> Image<Vec<u8>, 3> {
    let m = FONT.metrics(&[]);
    let sz = ppem * (m.max_width / m.units_per_em as f32);
    let mut i = Image::alloc(w as _, h as _);
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
                i.as_mut().overlay_at(
                    &Image::<Box<[u8]>, 4>::from(
                        Image::<_, 1>::build(
                            x.placement.width,
                            x.placement.height,
                        )
                        .buf(&*x.data),
                    )
                    .as_ref(),
                    4 + ((j as f32 * sz) + x.placement.left as f32) as u32,
                    ((k as f32 * (ppem * 1.25)) as u32)
                        .saturating_sub(x.placement.top as u32),
                    // x.placement.height - x.placement.top as u32,
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

use fimg::{Image, OverlayAt};
// let x = b"echo -e \"\x1b(0lqqqk\nx   \x1b(Bx\nmqqqj";
// let x = String::from_utf8_lossy(&x);
// println!("{}", x);
#[test]
fn t() {
    let f =
        FontRef::from_index(&include_bytes!("../CascadiaCode.ttf")[..], 0)
            .unwrap();

    dbg!(f.attributes());
    let m = f.metrics(&[]);
    dbg!(m);
    let ppem = 30.0;
    let sz = ppem * (m.max_width / m.units_per_em as f32);
    dbg!(
        &mut ScaleContext::new()
            .builder(f)
            .size(15.0)
            .build()
            .scale_outline(f.charmap().map('a'))
            .unwrap()
            .len()
    );
    let mut grid = Image::<_, 4>::alloc(2000, 150);
    unsafe { grid.chunked_mut().for_each(|x| *x = [0, 0, 0, 255]) };
    for (letter, i) in "the quick brown fox jumped over the lazy dog"
        .chars()
        .zip(0..)
    {
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
                ((i * sz as i32) + x.placement.left) as _,
                ppem as u32 - x.placement.top as u32,
                // x.placement.height - x.placement.top as u32,
            );
        }
    }
    grid.show();
}
