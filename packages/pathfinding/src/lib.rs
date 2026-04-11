use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn manhattan_path(start: u32, end: u32, width: u32) -> Vec<u32> {
    if width == 0 {
        return Vec::new();
    }

    let mut path: Vec<u32> = Vec::new();
    let mut cx = start % width;
    let mut cz = start / width;
    let tx = end % width;
    let tz = end / width;

    path.push(start);

    while cx != tx {
        if cx < tx {
            cx += 1;
        } else {
            cx -= 1;
        }
        path.push(cz * width + cx);
    }

    while cz != tz {
        if cz < tz {
            cz += 1;
        } else {
            cz -= 1;
        }
        path.push(cz * width + cx);
    }

    path
}
