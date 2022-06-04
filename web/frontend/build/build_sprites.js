#!/usr/bin/env node

const spritezero = require('@mapbox/spritezero');
const fs = require('fs');
const d3q = require('d3-queue').queue;
const path = require('path');
const stringify = require('json-stable-stringify');

const argv = require('minimist')(process.argv.slice(2), {
    boolean: ['retina', 'unique', 'h', 'help']
});

console.log('Building sprites.');

function filepaths (dir) {
    return fs.readdirSync(dir)
        .filter((d) => !d.match(/^\./))
        .map((d) => path.join(dir, d));
}

function showHelp () {
  console.log(`
  spritezero
  Generate sprite sheets for maps and the web using SVG files as input
  Usage
   <inputdir> <output>
  Example
  spritezero maki/ maki
  `);
}

if (argv.help || argv._.length < 2) {
    showHelp();
    /* istanbul ignore next */
    process.exit(1);
}

const unique = !!argv.unique;
const ratio = argv.retina
    ? 2
    : parseFloat(argv.ratio || 1);


const input = argv._[0];
const outfile = argv._[1];

function loadFile (file, callback) {
    fs.readFile(file, (err, res) =>
        callback(err, {
            svg: res,
            id: path.basename(file).replace('.svg', '')
        })
    );
}

function sortById (a, b) {
    return b.id < a.id;
}

const q = d3q(16);

filepaths(input).forEach((file) =>
    path.extname(file).toLowerCase() === '.svg'
    ? q.defer(loadFile, file)
    : null
);

q.awaitAll((err, buffers) => {
    if (err) throw err;

    buffers.sort(sortById);

    function saveLayout (err, formattedLayout) {
        if (err) throw err;
        fs.writeFile(outfile + '.json', stringify(formattedLayout, {space: '  '}), 'utf8', (err) => {
            if (err) throw err;
        });
    }

    function saveImage (err, layout) {
        if (err) throw err;
        spritezero.generateImage(layout, (err, image) => {
            if (err) throw err;
            fs.writeFile(outfile + '.png', image, (err) => {
                if (err) throw err;
            });
        });
    }

    const genLayout = unique ? spritezero.generateLayoutUnique : spritezero.generateLayout;
    genLayout({ imgs: buffers, pixelRatio: ratio, format: true }, saveLayout);
    genLayout({ imgs: buffers, pixelRatio: ratio, format: false }, saveImage);
});
