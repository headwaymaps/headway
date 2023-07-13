#!/usr/bin/env node

var spritezero = require('@mapbox/spritezero');
var fs = require('fs');
var queue = require('queue-async');
var path = require('path');
var stringify = require('json-stable-stringify');
var argv = require('minimist')(process.argv.slice(2), {
    boolean: ['retina', 'unique', 'h', 'help']
});

console.log('Building sprites.');

function filepaths (dir) {
    return fs.readdirSync(dir)
        .filter(function (d) {
            return !d.match(/^\./);
        })
        .map(function (d) {
            return path.join(dir, d);
        });
}

function showHelp () {
  const message = `
spritezero
Generate sprite sheets for maps and the web using SVG files as input
Usage
 <output> <inputdir>
Example
spritezero maki maki/
`;
  console.log(message);
}

if (argv.help || argv._.length < 2) {
    showHelp();
    /* istanbul ignore next */
    process.exit(1);
}

var ratio = 1;
var unique = false;

if (argv.retina) {
    ratio = 2;
} else if (argv.ratio) {
    ratio = parseFloat(argv.ratio);
}

if (argv.unique) {
    unique = true;
}

var outfile = argv._[0];
var input = argv._[1];

function loadFile (file, callback) {
    fs.readFile(file, function (err, res) {
        return callback(err, {
            svg: res,
            id: path.basename(file).replace('.svg', '')
        });
    });
}

function sortById (a, b) {
    return b.id < a.id;
}

var q = queue(16);

filepaths(input).forEach(function (file) {
    q.defer(loadFile, file);
});

q.awaitAll(function (err, buffers) {
    if (err) throw err;

    buffers.sort(sortById);

    function saveLayout (err, formattedLayout) {
        if (err) throw err;
        fs.writeFile(outfile + '.json', stringify(formattedLayout, {space: '  '}), 'utf8', function (err) {
            if (err) throw err;
        });
    }

    function saveImage (err, layout) {
        if (err) throw err;
        spritezero.generateImage(layout, function (err, image) {
            if (err) throw err;
            fs.writeFile(outfile + '.png', image, function (err) {
                if (err) throw err;
            });
        });
    }

    var genLayout = unique ? spritezero.generateLayoutUnique : spritezero.generateLayout;
    genLayout({ imgs: buffers, pixelRatio: ratio, format: true }, saveLayout);
    genLayout({ imgs: buffers, pixelRatio: ratio, format: false }, saveImage);
});
