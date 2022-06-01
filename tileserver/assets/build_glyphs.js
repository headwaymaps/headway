#!/usr/bin/env node

var fontnik = require('fontnik');
var path = require('path');
var fs = require('fs');
var queue = require('queue-async');

if (process.argv.length !== 4) {
    console.log('Usage:');
    console.log('  build-glyphs <fontstack path> <output dir>');
    console.log('');
    console.log('Example:');
    console.log('  build-glyphs ./fonts/open-sans/OpenSans-Regular.ttf ./glyphs');
    process.exit(1);
}

var fontstack = fs.readFileSync(process.argv[2]);
var dir = path.resolve(process.argv[3]);

if (!fs.existsSync(dir)) {
    console.warn('Error: Directory %s does not exist', dir);
    process.exit(1);
}

var q = queue(Math.max(4, require('os').cpus().length));
var queue = [];
for (var i = 0; i < 65536; (i = i + 256)) {
    q.defer(writeGlyphs, {
        font: fontstack,
        start: i,
        end: Math.min(i + 255, 65535)
    });
}

function writeGlyphs(opts, done) {
    fontnik.range(opts, function(err, zdata) {
        if (err) {
            console.warn(err.toString());
            process.exit(1);
        }
        fs.writeFileSync(dir + '/' + opts.start + '-' + opts.end + '.pbf', zdata);
        done();
    });
}
