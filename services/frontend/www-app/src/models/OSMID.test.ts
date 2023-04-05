import { expect, test } from '@jest/globals';
import OSMID from './OSMID';

test('OSMID.deserialize', () => {
  expect(OSMID.deserialize('way/206623301').isWay()).toBe(true);
  expect(OSMID.deserialize('way/206623301').isNode()).toBe(false);
  expect(OSMID.deserialize('way/206623301').isRelation()).toBe(false);
  expect(OSMID.deserialize('way/206623301').idNumber).toBe(206623301);
  expect(OSMID.deserialize('way/206623301').idType).toBe('way');
});
