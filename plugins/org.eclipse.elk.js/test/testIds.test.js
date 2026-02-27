/*******************************************************************************
 * Ported from elkjs — Copyright (c) 2017 Kiel University and others.
 * SPDX-License-Identifier: EPL-2.0
 *******************************************************************************/
import { describe, it, expect } from 'vitest';
import ELK from '../js/index.js';

const elk = new ELK();

describe('IDs', () => {
  describe('#layout(...)', () => {

    it('should return no error if id is a string', async () => {
      await elk.layout({ "id": "x" });
    });

    it('should return no error if id is an integer', async () => {
      await elk.layout({ "id": 2 });
    });

    it('should return an error if id is not present', async () => {
      await expect(elk.layout({ })).rejects.toThrow();
    });

    it('should return an error if id is a non-integral number', async () => {
      await expect(elk.layout({ "id": 1.2 })).rejects.toThrow();
    });

    it('should return an error if id is an array', async () => {
      await expect(elk.layout({ "id": [] })).rejects.toThrow();
    });

    it('should return an error if id is an object', async () => {
      await expect(elk.layout({ "id": {} })).rejects.toThrow();
    });

    it('should return an error if id is a boolean', async () => {
      await expect(elk.layout({ "id": true })).rejects.toThrow();
    });

  });
});
