import Area from "./Area";

test("city should include country code", () => {
  const area = new Area("Seattle", ["US"], ["us/wa/king"]);
  expect(area.importsConfig()).toEqual({
    whosonfirst: {
      datapath: "/data/whosonfirst",
      importPostalcodes: true,
      countryCode: ["US"],
    },
    openaddresses: {
      datapath: "/data/openaddresses",
      files: ["us/wa/king"],
    },
  });
});

test("city should exclude openaddresses altogether if no files specified", () => {
  const area = new Area("Seattle", ["US"], []);
  expect(area.importsConfig()).toEqual({
    whosonfirst: {
      datapath: "/data/whosonfirst",
      importPostalcodes: true,
      countryCode: ["US"],
    },
  });
});

test("planet should exclude country code so as to import everythign", () => {
  const area = new Area("planet", ["ALL"], []);
  expect(area.importsConfig()).toEqual({
    whosonfirst: {
      datapath: "/data/whosonfirst",
      importPostalcodes: true,
    },
    openaddresses: {
      datapath: "/data/openaddresses",
    },
  });
});
