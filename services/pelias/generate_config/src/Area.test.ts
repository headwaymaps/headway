import Area from "./Area";

test("city should include country code", () => {
  const area = new Area("Seattle", ["US"]);
  expect(area.importsConfig().whosonfirst).toEqual({
    datapath: "/data/whosonfirst",
    importPostalcodes: true,
    countryCode: ["US"],
  });
});

test("city should specify openaddresses files", () => {
  const area = new Area("Seattle", ["US"]);
  expect(area.importsConfig().openaddresses).toEqual({
    datapath: "/data/openaddresses",
    files: ["bbox_addresses"],
  });
});

test("planet should exclude specific openaddresses files so as to import everything", () => {
  const area = new Area("planet", ["ALL"]);
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
