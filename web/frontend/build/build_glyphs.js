#!/usr/bin/env node

const fontnik = require('fontnik');
const path = require('path');
const fs = require('fs');
const d3q = require('d3-queue');
const mkdirp = require('mkdirp');

if (process.argv.length !== 4) {
    console.log('Usage:');
    console.log('  build-glyphs <fontstack_path> <output_dir>');
    console.log('');
    console.log('Example:');
    console.log('  build-glyphs ./fonts/open-sans/OpenSans-Regular.ttf ./glyphs');
    process.exit(1);
}

const src = process.argv[2];
const fontstack = fs.readFileSync(src);
const baseDir = path.resolve(process.argv[3]);
if (!fs.existsSync(baseDir)) {
    console.error(`Error: Directory ${baseDir} does not exist`);
    process.exit(1);
}
if (!fs.lstatSync(baseDir).isDirectory()) {
    console.error(`Error: ${baseDir} is not a directory`);
    process.exit(1);
}
const dir = path.join(baseDir, path.basename(src, path.extname(src)));
mkdirp.sync(dir);

const q = d3q.queue(Math.max(4, require('os').cpus().length));
for (let i = 0; i < 65536; (i = i + 256)) {
    q.defer(writeGlyphs, {
        font: fontstack,
        start: i,
        end: Math.min(i + 255, 65535)
    });
}

function writeGlyphs(opts, done) {
    fontnik.range(opts, (err, zdata) => {
        if (err) {
            console.error(err.toString());
            return process.exit(1);
        }
        fs.writeFileSync(path.join(dir, `${opts.start}-${opts.end}.pbf`), zdata);
        done();
    });
}
