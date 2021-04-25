function hue2rgb(p, q, t) {
    if (t < 0) t += 1;
    if (t > 1) t -= 1;
    if (t < 1 / 6) return p + (q - p) * 6 * t;
    if (t < 1 / 2) return q;
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
    return p;
}

function hslToRgb(h, s, l) {
    let r, g, b;

    if (s === 0) {
        r = g = b = l;
    } else {
        const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
        const p = 2 * l - q;
        r = hue2rgb(p, q, h + 1 / 3);
        g = hue2rgb(p, q, h);
        b = hue2rgb(p, q, h - 1 / 3);
    }

    return {
        r: Math.round(r * 255),
        g: Math.round(g * 255),
        b: Math.round(b * 255),
        a: 255,
    };
}

function get_color(min, max, val) {
    const left_span = max - min;
    const value_scaled = val - min / left_span;
    const h = 1 - value_scaled;
    const s = 1;
    const l = value_scaled / 2;
    return hslToRgb(h, s, l);
}

export function compute_color_array(n) {
    const color_array = [];
    for (let i = 0; i < n; i++) {
        color_array.push(get_color(n, i, 0));
    }
    return color_array;
}
