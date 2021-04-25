import { Cell, Config, DrawMode, Lattice } from "wasm-lattice-boltzmann";
// @ts-ignore
import { memory } from "wasm-lattice-boltzmann/wasm_lattice_boltzmann_bg.wasm";
import { compute_color_array } from "./color";

interface BoltzmannProps {
    boltzcanvas: string;
    vectorcanvas: string;
    particlecanvas: string;
    barriercanvas: string;
    width: number;
    height: number;
    steps_per_frame?: number;
}

export class Boltzmann {
    static four9ths = 4.0 / 9.0;
    static one9th = 1.0 / 9.0;
    static one36th = 1.0 / 36.0;
    static num_colors = 400;
    static color_array = compute_color_array(Boltzmann.num_colors);
    boltzcanvas: HTMLCanvasElement;
    vectorcanvas: HTMLCanvasElement;
    particlecanvas: HTMLCanvasElement;
    barriercanvas: HTMLCanvasElement;
    boltzctx: CanvasRenderingContext2D;
    vectorctx: CanvasRenderingContext2D;
    particlectx: CanvasRenderingContext2D;
    barrierctx: CanvasRenderingContext2D;
    image: ImageData;
    canvasWidth: number;
    canvasHeight: number;
    image_data: Uint8ClampedArray;
    image_width: number;
    width: number;
    height: number;
    px_per_node: number;
    steps_per_frame: number;
    flow_speed: number;
    flow_vectors: boolean;
    draw_mode: DrawMode;
    viscosity: number;
    animation_id: number | null;
    stopped: boolean;
    lattice: Lattice;
    constructor(props: BoltzmannProps) {
        this.boltzcanvas = document.getElementById(
            props.boltzcanvas
        ) as HTMLCanvasElement;
        this.vectorcanvas = document.getElementById(
            props.vectorcanvas
        ) as HTMLCanvasElement;
        this.particlecanvas = document.getElementById(
            props.particlecanvas
        ) as HTMLCanvasElement;
        this.barriercanvas = document.getElementById(
            props.barriercanvas
        ) as HTMLCanvasElement;
        this.boltzctx = this.boltzcanvas.getContext("2d");
        this.vectorctx = this.vectorcanvas.getContext("2d");
        this.particlectx = this.particlecanvas.getContext("2d");
        this.barrierctx = this.barriercanvas.getContext("2d");
        this.vectorctx.strokeStyle = "red";
        this.vectorctx.fillStyle = "red";
        this.particlectx.strokeStyle = "black";
        this.particlectx.fillStyle = "black";
        this.barrierctx.fillStyle = "yellow";
        this.canvasWidth = this.boltzcanvas.width;
        this.canvasHeight = this.boltzcanvas.height;
        this.image = this.boltzctx.createImageData(
            this.canvasWidth,
            this.canvasHeight
        );
        this.image_data = this.image.data;
        this.image_width = this.image.width;

        this.width = props.width;
        this.height = props.height;
        this.px_per_node = Math.floor(this.boltzcanvas.width / this.width);
        this.steps_per_frame = props.steps_per_frame || 5;
        this.flow_speed = 0.0;
        this.flow_vectors = false;
        this.draw_mode = 0;
        this.viscosity = 0.27;

        this.animation_id = null;
        this.stopped = true;

        const config = Config.new(
            this.width,
            this.height,
            this.steps_per_frame,
            this.flow_speed,
            this.draw_mode,
            this.viscosity
        );
        this.lattice = Lattice.new(config);

        const size = this.width * this.height;

        this.renderLoop = this.renderLoop.bind(this);
        this.drawFrame();
        this.registerEventListeners();
    }
    getMemoryWindow(name: string, type: any, size: number) {
        // TODO
        // @ts-ignore
        const ptr = this.lattice[name]();
        return new type(memory.buffer, ptr, size);
    }
    draw_flow_particle(x: number, y: number) {
        this.particlectx.beginPath();
        this.particlectx.arc(
            x * this.px_per_node,
            y * this.px_per_node,
            1,
            0,
            2 * Math.PI,
            false
        );
        this.particlectx.fill();
        this.particlectx.closePath();
    }

    draw_flow_vector(x: number, y: number, ux: number, uy: number) {
        const scale = 200;
        const xpx = x * this.px_per_node;
        const ypx = y * this.px_per_node;
        this.vectorctx.beginPath();
        this.vectorctx.moveTo(xpx, ypx);
        this.vectorctx.lineTo(
            Math.round(xpx + ux * this.px_per_node * scale),
            ypx + uy * this.px_per_node * scale
        );
        this.vectorctx.stroke();
        this.vectorctx.beginPath();
        this.vectorctx.arc(xpx, ypx, 1, 0, 2 * Math.PI, false);
        this.vectorctx.fill();
        this.vectorctx.closePath();
    }

    drawFrame() {
        let x, y, l;
        if (this.flow_vectors) {
            this.vectorctx.clearRect(0, 0, this.canvasWidth, this.canvasHeight);
        }

        const latticeWidth = this.width;
        const latticeHeight = this.height;
        const size = latticeWidth * latticeHeight;

        const flowSize = this.lattice.flow_size();
        const flow_particles_x = this.getMemoryWindow(
            "flow_particles_x",
            Float64Array,
            flowSize
        );
        const flow_particles_y = this.getMemoryWindow(
            "flow_particles_y",
            Float64Array,
            flowSize
        );
        if (flow_particles_x.length > 0) {
            this.particlectx.clearRect(
                0,
                0,
                this.canvasWidth,
                this.canvasHeight
            );
            for (x = 0, l = flow_particles_x.length; x < l; x++) {
                this.draw_flow_particle(
                    flow_particles_x[x],
                    flow_particles_y[x]
                );
            }
        }
        // if (new_barrier) {
        //     barrierctx.clearRect(0, 0, canvasWidth, canvasHeight);
        //     draw_barriers(barrierctx);
        //     new_barrier = false;
        // }

        const draw_mode = this.lattice.draw_mode();

        const Lux = this.getMemoryWindow("ux", Float64Array, size);
        const Luy = this.getMemoryWindow("uy", Float64Array, size);
        const Lbarrier = this.getMemoryWindow("barrier", Uint8Array, size);
        const Ldensity =
            draw_mode === DrawMode.Density
                ? this.getMemoryWindow("density", Float64Array, size)
                : undefined;
        const Lcurl =
            draw_mode === DrawMode.Curl
                ? this.getMemoryWindow("curl", Float64Array, size)
                : undefined;

        for (x = 0; x < latticeWidth; x++) {
            for (y = 0; y < latticeHeight; y++) {
                const idx = y * latticeWidth + x;
                let color_index;
                if (!Lbarrier[idx]) {
                    color_index = 0;
                    const ux = Lux[idx];
                    const uy = Luy[idx];

                    if (this.flow_vectors && x % 10 === 0 && y % 10 === 0) {
                        // Draw flow vectors every tenth node.
                        this.draw_flow_vector(x, y, ux, uy);
                    }

                    // There are a lot of magic numbers ahead.
                    // They are primarily experimentally derived values chosen
                    // to produce aesthetically pleasing results.
                    switch (this.draw_mode) {
                        case DrawMode.Speed: {
                            // Speed
                            const speed = Math.sqrt(
                                Math.pow(ux, 2) + Math.pow(uy, 2)
                            );
                            color_index = Math.floor(
                                (speed + 0.21) * Boltzmann.num_colors
                            );
                            break;
                        }
                        case DrawMode.XVelocity: {
                            color_index = Math.floor(
                                (ux + 0.21052631578) * Boltzmann.num_colors
                            );
                            break;
                        }
                        case DrawMode.YVelocity: {
                            color_index = Math.floor(
                                (uy + 0.21052631578) * Boltzmann.num_colors
                            );
                            break;
                        }
                        case DrawMode.Density: {
                            const dens = Ldensity[idx];
                            color_index = Math.floor(
                                (dens - 0.75) * Boltzmann.num_colors
                            );
                            break;
                        }
                        case DrawMode.Curl: {
                            const curl = Lcurl[idx];
                            color_index = Math.floor(
                                (curl + 0.25196850393) * Boltzmann.num_colors
                            );
                            break;
                        }
                        default: {
                            // Draw nothing. This mode is useful when flow vectors or particles are turned on.
                            break;
                        }
                    }
                    if (color_index >= Boltzmann.num_colors) {
                        color_index = Boltzmann.num_colors - 1;
                    } else if (color_index < 0) {
                        color_index = 0;
                    }
                    const color = Boltzmann.color_array[color_index];
                    // draw_square inlined for performance
                    for (
                        let ypx = y * this.px_per_node;
                        ypx < (y + 1) * this.px_per_node;
                        ypx++
                    ) {
                        for (
                            let xpx = x * this.px_per_node;
                            xpx < (x + 1) * this.px_per_node;
                            xpx++
                        ) {
                            const index = (xpx + ypx * this.image_width) * 4;
                            this.image_data[index] = color.r;
                            this.image_data[index + 1] = color.g;
                            this.image_data[index + 2] = color.b;
                            this.image_data[index + 3] = color.a;
                        }
                    }
                }
            }
        }
        this.boltzctx.putImageData(this.image, 0, 0);
    }

    renderLoop() {
        this.lattice.update();
        this.drawFrame();
        if (!this.stopped) {
            this.animation_id = requestAnimationFrame(this.renderLoop);
        } else {
            this.animation_id = null;
        }
    }

    moveHelper(newX: number, newY: number, oldX: number, oldY: number) {
        const radius = 5;
        let dx = (newX - oldX) / this.px_per_node / this.steps_per_frame;
        let dy = (newY - oldY) / this.px_per_node / this.steps_per_frame;
        // Ensure that push isn't too big
        if (Math.abs(dx) > 0.1) {
            dx = (0.1 * Math.abs(dx)) / dx;
        }
        if (Math.abs(dy) > 0.1) {
            dy = (0.1 * Math.abs(dy)) / dy;
        }
        // Scale from canvas coordinates to lattice coordinates
        const lattice_x = Math.floor(newX / this.px_per_node);
        const lattice_y = Math.floor(newY / this.px_per_node);
        const size = this.width * this.height;

        const Lbarrier = this.getMemoryWindow("barrier", Uint8Array, size);
        const Ldensity = this.getMemoryWindow("density", Float64Array, size);

        for (let x = -radius; x <= radius; x++) {
            for (let y = -radius; y <= radius; y++) {
                // Push in circle around cursor. Make sure coordinates are in bounds.
                if (
                    lattice_x + x >= 0 &&
                    lattice_x + x < this.width &&
                    lattice_y + y >= 0 &&
                    lattice_y + y < this.height &&
                    !Lbarrier[(lattice_y + y) * this.width + (lattice_x + x)] &&
                    Math.sqrt(x * x + y * y) < radius
                ) {
                    const idx = (lattice_y + y) * this.width + (lattice_x + x);
                    const ux = dx;
                    const uy = dy;
                    const rho = Ldensity[idx];
                    const ux3 = 3 * ux;
                    const uy3 = 3 * -uy;
                    const ux2 = ux * ux;
                    const uy2 = -uy * -uy;
                    const uxuy2 = 2 * ux * -uy;
                    const u2 = ux2 + uy2;
                    const u215 = 1.5 * u2;

                    const new_cell = Cell.new(
                        Boltzmann.four9ths * rho * (1 - u215),
                        Boltzmann.one9th * rho * (1 + ux3 + 4.5 * ux2 - u215),
                        Boltzmann.one9th * rho * (1 + uy3 + 4.5 * uy2 - u215),
                        Boltzmann.one9th * rho * (1 - ux3 + 4.5 * ux2 - u215),
                        Boltzmann.one9th * rho * (1 - uy3 + 4.5 * uy2 - u215),
                        Boltzmann.one36th *
                            rho *
                            (1 + ux3 + uy3 + 4.5 * (u2 + uxuy2) - u215),
                        Boltzmann.one36th *
                            rho *
                            (1 - ux3 + uy3 + 4.5 * (u2 - uxuy2) - u215),
                        Boltzmann.one36th *
                            rho *
                            (1 - ux3 - uy3 + 4.5 * (u2 + uxuy2) - u215),
                        Boltzmann.one36th *
                            rho *
                            (1 + ux3 - uy3 + 4.5 * (u2 - uxuy2) - u215)
                    );
                    this.lattice.set_cell(new_cell, idx);
                }
            }
        }
    }

    registerEventListeners() {
        // Register left click
        this.boltzcanvas.addEventListener(
            "mousedown",
            (e) => this.mousedownListener(e),
            false
        );
        // boltzcanvas.addEventListener('touchstart', touchdownListener, false);

        // Register right click
        // boltzcanvas.addEventListener('contextmenu', place_barrier, false);

        // Register dropdown
        const drawoptions = document.getElementById("drawmode");
        drawoptions.addEventListener(
            "change",
            (e: MouseEvent) => {
                this.lattice.set_draw_mode(
                    parseInt((e.target as HTMLInputElement).value, 10)
                );
            },
            false
        );

        // Register sliders
        let viscoslider = document.getElementById("viscosity");
        viscoslider.addEventListener(
            "input",
            (e) =>
                this.lattice.set_viscosity(
                    parseFloat((e.target as HTMLInputElement).value) / 100
                ),
            false
        );
        // Register checkboxes
        const flowvector = document.getElementById("flowvectors");
        flowvector.addEventListener(
            "click",
            (e) => {
                this.flow_vectors = (e.target as HTMLInputElement).checked;
                this.drawFrame();
            },
            false
        );

        const flowparticle = document.getElementById("flowparticles");
        flowparticle.addEventListener(
            "click",
            (e) => {
                if ((e.target as HTMLInputElement).checked) {
                    this.lattice.init_flow_particles();
                } else {
                    this.lattice.clear_flow_particles();
                    this.particlectx.clearRect(
                        0,
                        0,
                        this.canvasWidth,
                        this.canvasHeight
                    ); // Clear
                }
            },
            false
        );

        // Register start/stop
        const startbutton = document.getElementById("play");
        startbutton.addEventListener(
            "click",
            () => {
                this.stopped = !this.stopped;
                if (!this.stopped) {
                    this.renderLoop();
                }
            },
            false
        );
        // Register reset
        // var resetbutton = document.getElementById('reset');
        // resetbutton.addEventListener('click', reset, false);

        // Register clear barriers
        // var clear = document.getElementById('clearbarriers');
        // clear.addEventListener('click', clear_barriers, false);

        // Register flow speed slider
        const flow_speed = document.getElementById("flow-speed");
        flow_speed.addEventListener(
            "input",
            (e) =>
                this.lattice.set_flow_speed(
                    parseFloat((e.target as HTMLInputElement).value) / 833
                ),
            false
        );

        const speed = document.getElementById("speed");
        speed.addEventListener(
            "input",
            (e) => {
                this.lattice.set_steps_per_frame(
                    parseInt((e.target as HTMLInputElement).value, 10)
                );
            },
            false
        );
    }

    mousedownListener(e: MouseEvent) {
        const button = e.which || e.button;
        if (button !== 1) {
            // Only capture left click
            return;
        }
        if (!this.animation_id) {
            // Don't capture if stopped
            return;
        }
        let oldX = e.offsetX;
        let oldY = e.offsetY;

        const moveListener = (e: MouseEvent) => {
            const newX = e.offsetX;
            const newY = e.offsetY;
            this.moveHelper(newX, newY, oldX, oldY);
            oldX = newX;
            oldY = newY;
        };

        const mouseupListener = () => {
            this.boltzcanvas.removeEventListener(
                "mousemove",
                moveListener,
                false
            );
            this.boltzcanvas.removeEventListener(
                "mouseup",
                mouseupListener,
                false
            );

            this.boltzcanvas.removeEventListener(
                "touchmove",
                moveListener,
                false
            );
            document.body.removeEventListener(
                "touchend",
                mouseupListener,
                false
            );
        };

        this.boltzcanvas.addEventListener("mousemove", moveListener, false);
        this.boltzcanvas.addEventListener("mouseup", mouseupListener, false);

        this.boltzcanvas.addEventListener("touchmove", moveListener, false);
        document.body.addEventListener("touchend", mouseupListener, false);
    }
}
