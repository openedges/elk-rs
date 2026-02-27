/*******************************************************************************
 * Ported from elkjs — Copyright (c) 2017 Kiel University and others.
 * SPDX-License-Identifier: EPL-2.0
 *******************************************************************************/
import { describe, it, expect } from 'vitest';
import ELK from '../js/index.js';

const elk = new ELK();

describe('Parameters', () => {
  describe('#layout()', () => {

    it('should be rejected if graph is missing', async () => {
      await expect(elk.layout()).rejects.toThrow();
    });

    it('should succeed if a graph is specified ', async () => {
      await elk.layout({"id": 2});
    });

  });
});
