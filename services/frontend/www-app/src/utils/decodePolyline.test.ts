import { expect, test } from '@jest/globals';

import { decodePolyline } from './decodePolyline';

test('decode', () => {
  const input =
    'wpxvyA~w~ihFxIDAhDEnQCnA]nC_BdJ]Va@hASvCorD_@k~DnAkNDoxFAke@a@iKq@cG}AuEaCqG}EwMcQiI{K_CmDcAkAqNgPoYcVqIeFaGiD}IaEwcAi_@yLuDaIeCcD_AeYaIyKa@aEMyOmDmSyH}t@u[cEiCuFkFkDmDuAyAaSoJcNuD}OcAwVMoU?klBCmEUeDqA_C}Bg@{AAuDqHg@aAG_Uf@_YhBqMx@_I`@cM`@oAAiJCiIVgCDqYf@oJJeFP}FFwYPaKTgDDuCByGIyJQeNGeE\\_@DmD`@gCp@cATaB^mDfA_DfAwD`B_MnHcJYmCbB}BtAq_@zYyFrFyBlBw_@~\\cFnEsBsG{Mmb@eBmFcBqFqF}PkAoDaBcFmHyToBaG_CtByJvIoUjS}CnCmC~Bid@z`@{ApAaBxAud@da@YTaBvAwChCwc@j`@iB`BkCbCsDjDeUxR{X|UcCrBqC`Cyg@zc@qIpHOLyEfEoC~B{CnCkPxNaGbFyU~RwCbCcCvBmSrQuCfCwB{CeCoDoKgOEIeMsQoDcFmBmCsAoBwNqSwAoBsK_OcDqEwD{EgAkBiJqL{F}H{GsJwB_DwAsBmCyDaOwSkNeS}@oA}CuE{@oAqHoKcE}FuEf@iEd@yTrB{BRqBRwGf@oE`@kKfAuANqMzAeCXkD`@gUrC_BRcCZc@x@{@dB_CtE{@dBiTtb@oBxDkCjF_HfNaJvQiSja@iBrDwBhEkh@vdA{AzC_AnBg@bAiYll@iAzBuAlB_BxAeBdAkBn@oBXoBBcA?mMCeEAcDAeQGqQGmIEqFAwFE_E?wCCuWIme@KsCAyDA{b@EsIA{@?yDAsFA{DA}C?}NBwO?g\\?yC@wDAiL@sF?aUByD?Y?eH@sCAqD?}FDiC?kA?oOQ_CAaCAeGAuEA_BAmD?wCCyi@YgDCuEA{XImQGsCAqCA_H?sAAaj@Iwz@MeVCiSAeBAiD?qe@MeEAaFA}WI_OEiA?qAAoe@M_LEkA?_C?uC?aEAiF?oLA}EA{B?{D@iH@uE@mb@GeECg`AMqFAy@?_QEoC@oCPgANgANmCl@mC~@iClAeD|AkFhCeGvCaUtKs@\\sHtDgb@lSyL`G{CvAcErBsQ~IgKdFkN~Ec`AhTcB^ib@lJwMvCq@|DUvASpAGZ}CzAyLrCsQhEeE`AuCx@qCjAmCzAiClBuLtI}AxAwA`BuAfBqAnBgX`c@`@vYeDbR]n@_B[O[o@bAy@v@{PbE}B^{Cd@eMlD{@z@eATuCToQtA}ZfAuPRel@Z}_@PexIC';
  const result = decodePolyline(input, 6);

  expect(result.length).toBe(350);
  const firstCoord = result[0]!;
  expect(firstCoord[0]!).toBeCloseTo(-122.339216);
  expect(firstCoord[1]!).toBeCloseTo(47.575836);

  const lastCoord = result[349]!;
  expect(lastCoord[0]!).toBeCloseTo(-122.347199);
  expect(lastCoord[1]!).toBeCloseTo(47.651048);
});
