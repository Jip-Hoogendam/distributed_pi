{
    //noise adapted from 
    // 2D Noise based on Morgan McGuire @morgan3d
    // https://www.shadertoy.com/view/4dS3Wd
    function xy_random(x ,y){
        let a = Math.sin((x * 12.9898 + y * 78.233) * 43758.5453123);
        return a - Math.floor(a);
    }

    // Linear interpolation
    function lerp(a, b, t) {
        return a * (1 - t) + b * t;
    }

    function noise(x , y){
        let i_x = Math.floor(x);
        let i_y = Math.floor(y);
        let f_x = x - Math.floor(x);
        let f_y = y - Math.floor(y);

        //four corners in 2D of a tile

        let a = xy_random(i_x, i_y);
        let b = xy_random(i_x + 1.0, i_y + 0.0);
        let c = xy_random(i_x + 0.0, i_y + 1.0);
        let d = xy_random(i_x + 1.0, i_y + 1.0);


        // Smooth Interpolation
        // Cubic Hermine Curve.  Same as SmoothStep() of fragment shaders
        let u_x = f_x*f_x*(3.0-2.0*f_x);
        let u_y = f_y*f_y*(3.0-2.0*f_y);

        // Mix 4 corners
        let mix_ab = lerp(a, b, u_x);
        let mix_cd = lerp(c, d, u_x);
        return lerp(mix_ab, mix_cd, u_y);
    }



    const canvas = document.createElement("canvas");

    gl = canvas.getContext("webgl2");

    canvas.id = "background"


    document.body.appendChild(canvas);

    // Compile vertex shader
    var vs = gl.createShader(gl.VERTEX_SHADER);
    var vsSource = `#version 300 es
                    in vec3 pos;

                    out vec2 coord;
                    void main() {
                        coord = pos.xy;
                        gl_Position = vec4(pos, 1.0);
                    }`;
    gl.shaderSource(vs, vsSource);
    gl.compileShader(vs);

    // Compile fragment shader
    var fs = gl.createShader(gl.FRAGMENT_SHADER);

    const fragment_shaders = [
        "wavy_grid",
        "cubic_slide",
        "scrolling_lines",
        "grid_walk",
        "bad_acid",
        "glitch_grid",
        "sludge_drip"
    ]

    let random_shader_pointer = Math.ceil(Math.random() * fragment_shaders.length - 1);


    fetch('/scripts/background/' + fragment_shaders[random_shader_pointer] +'.frag')
    .then(response => response.text())
    .then((fsSource) => {
        gl.shaderSource(fs, fsSource);
        gl.compileShader(fs);

        // Combine the two shaders into a program
        var program = gl.createProgram();
        gl.attachShader(program, vs);
        gl.attachShader(program, fs);
        gl.linkProgram(program);
        gl.useProgram(program);

        // Create an arrayBuffer and fill it with data
        var buffer = gl.createBuffer()
        gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
        gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([
            -1,  1, 0,
            1,  1, 0,
            -1, -1, 0,
            -1, -1, 0,
            1,  1, 0,
            1, -1, 0
            ]), gl.STATIC_DRAW);

        // Point the attribute pos to the arrayBuffer
        var attr = gl.getAttribLocation(program, "pos");
        gl.enableVertexAttribArray(attr);
        gl.vertexAttribPointer(attr, 3, gl.FLOAT, gl.FALSE, 0, 0); // 3 means: take 3 items from the buffer per vertex

        // Set the background color
        gl.clearColor(0.8, 0.8, 0.8, 1); // r, g, b, alpha

        
        let start_time = Date.now() - Math.random() * 100000;
        
        let xMotionLocation = gl.getUniformLocation(program, "x_motion");
        gl.uniform1f(xMotionLocation, 0);
        let yMotionLocation = gl.getUniformLocation(program, "y_motion");
        gl.uniform1f(yMotionLocation, 0);
        let timeLocation = gl.getUniformLocation(program, "time");
        gl.uniform1f(timeLocation, 0);
        let resolutionLocation = gl.getUniformLocation(program, "u_resolution");
        gl.uniform2f(resolutionLocation, canvas.width, canvas.height);

        let x_motion = (Math.random() - 0.5) * 100;
        let y_motion = (Math.random() - 0.5) * 100;


        function draw(){
            let screen_width = document.documentElement.clientWidth;
            let screen_height = document.documentElement.clientHeight;
            canvas.height = screen_height;
            canvas.width = screen_width;
            gl.viewport(0,0,gl.canvas.width, gl.canvas.height);
            // Actual drawing instructions
            gl.clear(gl.COLOR_BUFFER_BIT);
            gl.drawArrays(gl.TRIANGLES, 0, 6);
            let delta_time = (Date.now() - start_time) / 10000;

            x_motion += (noise(delta_time, 100) - 0.5)/ 500;
            y_motion += (noise(delta_time, 300) - 0.5) / 500;
            gl.uniform1f(xMotionLocation, x_motion);
            gl.uniform1f(yMotionLocation, y_motion);
            gl.uniform1f(timeLocation, delta_time);
            gl.uniform2f(resolutionLocation, canvas.width, canvas.height);
            requestAnimationFrame(draw);
        }

        draw();
    });
    
}