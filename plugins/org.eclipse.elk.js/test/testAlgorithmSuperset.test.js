import { describe, expect, it } from 'vitest';
import { fileURLToPath } from 'node:url';
import ELK from '../js/index.js';
import ELKApi from '../js/elk-api.js';
import { Worker as FakeWorker } from '../js/elk-worker.js';

const constructors = {
  direct: () => new ELK({ algorithms: ['layered'] }),
  fakeWorker: () => new ELKApi({ algorithms: ['layered'], workerFactory: () => new FakeWorker() }),
  workerThreads: () => new ELK({ algorithms: ['layered'],
    workerUrl: fileURLToPath(new URL('../js/elk-worker.js', import.meta.url)) }),
};

describe('Java algorithm superset in every JS mode', () => {
  for (const [mode, create] of Object.entries(constructors)) {
    it(`retains VertiFlex with the compatibility algorithms hint (${mode})`, async () => {
      const elk = create();
      try {
        const ids = (await elk.knownLayoutAlgorithms()).map(algorithm => algorithm.id);
        expect(ids).toContain('org.eclipse.elk.vertiflex');
        expect(ids).toContain('org.eclipse.elk.disco');
        const result = await elk.layout({ id: 'root',
          layoutOptions: { 'elk.algorithm': 'vertiflex' },
          children: [{ id: 'a', width: 30, height: 20 }, { id: 'b', width: 30, height: 20 }],
          edges: [{ id: 'ab', sources: ['a'], targets: ['b'] }],
        });
        expect(result.children.every(node => Number.isFinite(node.x) && Number.isFinite(node.y))).toBe(true);
        expect(result.children[0].y).not.toBe(result.children[1].y);
      } finally {
        elk.terminateWorker();
      }
    });
  }
});
