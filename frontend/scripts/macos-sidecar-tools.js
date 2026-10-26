const { spawnSync } = require('child_process');
const {
  chmodSync,
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  statSync,
} = require('fs');
const path = require('path');

const MACHO_64_MAGIC = 0xfeedfacf;
const FAT_MAGIC = 0xcafebabe;
const FAT_CIGAM = 0xbebafeca;
const FAT_MAGIC_64 = 0xcafebabf;
const FAT_CIGAM_64 = 0xbfbafeca;
const CPU_TYPES = {
  'aarch64-apple-darwin': 0x0100000c,
  'x86_64-apple-darwin': 0x01000007,
};

function verifyExecutable(filePath, label = path.basename(filePath)) {
  if (!existsSync(filePath) || !statSync(filePath).isFile()) {
    throw new Error(`Missing executable: ${label}`);
  }

  if ((statSync(filePath).mode & 0o111) === 0) {
    throw new Error(`Executable bit is missing: ${label}`);
  }
}

function verifyThinMachO(filePath, target) {
  const label = path.basename(filePath);
  verifyExecutable(filePath, label);
  const expectedCpuType = CPU_TYPES[target];
  if (!expectedCpuType) {
    throw new Error(`Unsupported macOS target: ${target}`);
  }

  const binary = readFileSync(filePath);
  const actualCpuType = readThinCpuType(binary, 0, binary.length, label);
  if (actualCpuType !== expectedCpuType) {
    throw new Error(`Mach-O architecture mismatch for ${label}: expected ${target}`);
  }
}

function verifyUniversalMachO(filePath) {
  const label = path.basename(filePath);
  verifyExecutable(filePath, label);
  const binary = readFileSync(filePath);
  if (binary.length < 8) {
    throw new Error(`Universal Mach-O header is missing for ${label}`);
  }

  const magic = binary.readUInt32BE(0);
  const fat64 = magic === FAT_MAGIC_64 || magic === FAT_CIGAM_64;
  const littleEndian = magic === FAT_CIGAM || magic === FAT_CIGAM_64;
  if (
    magic !== FAT_MAGIC
    && magic !== FAT_CIGAM
    && magic !== FAT_MAGIC_64
    && magic !== FAT_CIGAM_64
  ) {
    throw new Error(`Universal Mach-O header is invalid for ${label}`);
  }

  const entrySize = fat64 ? 32 : 20;
  const architectureCount = readUInt32(binary, 4, littleEndian);
  const tableEnd = 8 + architectureCount * entrySize;
  if (binary.length < tableEnd) {
    throw new Error(`Universal Mach-O architecture table is truncated for ${label}`);
  }

  const slices = [];
  for (let index = 0; index < architectureCount; index += 1) {
    const base = 8 + index * entrySize;
    const cpuType = readUInt32(binary, base, littleEndian);
    const offset = fat64
      ? readUInt64(binary, base + 8, littleEndian)
      : BigInt(readUInt32(binary, base + 8, littleEndian));
    const size = fat64
      ? readUInt64(binary, base + 16, littleEndian)
      : BigInt(readUInt32(binary, base + 12, littleEndian));

    if (size === 0n) {
      throw new Error(`Universal Mach-O ${label} has an empty slice`);
    }
    if (offset < BigInt(tableEnd)) {
      throw new Error(
        `Universal Mach-O ${label} slice begins before the complete architecture table`,
      );
    }

    const end = offset + size;
    if (end > BigInt(binary.length)) {
      throw new Error(`Universal Mach-O ${label} slice is out of bounds`);
    }

    slices.push({
      cpuType,
      offset: Number(offset),
      size: Number(size),
      end: Number(end),
    });
  }

  const sortedSlices = [...slices].sort((left, right) => left.offset - right.offset);
  for (let index = 1; index < sortedSlices.length; index += 1) {
    if (sortedSlices[index].offset < sortedSlices[index - 1].end) {
      throw new Error(`Universal Mach-O ${label} slices overlap`);
    }
  }

  const cpuTypes = new Set();
  for (const slice of slices) {
    if (cpuTypes.has(slice.cpuType)) {
      throw new Error(`Universal Mach-O ${label} contains a duplicate CPU slice`);
    }
    cpuTypes.add(slice.cpuType);

    const nestedCpuType = readThinCpuType(
      binary,
      slice.offset,
      slice.size,
      `${label} slice`,
    );
    if (nestedCpuType !== slice.cpuType) {
      throw new Error(`Universal Mach-O ${label} slice CPU type does not match its table entry`);
    }
  }

  for (const [target, cpuType] of Object.entries(CPU_TYPES)) {
    if (!cpuTypes.has(cpuType)) {
      throw new Error(`Universal Mach-O ${label} is missing ${target}`);
    }
  }
}

function readUInt32(binary, offset, littleEndian) {
  return littleEndian
    ? binary.readUInt32LE(offset)
    : binary.readUInt32BE(offset);
}

function readUInt64(binary, offset, littleEndian) {
  return littleEndian
    ? binary.readBigUInt64LE(offset)
    : binary.readBigUInt64BE(offset);
}

function readThinCpuType(binary, offset, size, label) {
  if (size < 8 || offset + 8 > binary.length) {
    throw new Error(`${label} has invalid Mach-O magic`);
  }

  if (binary.readUInt32LE(offset) === MACHO_64_MAGIC) {
    return binary.readUInt32LE(offset + 4);
  }
  if (binary.readUInt32BE(offset) === MACHO_64_MAGIC) {
    return binary.readUInt32BE(offset + 4);
  }
  throw new Error(`${label} has invalid Mach-O magic`);
}

function createFatWithLipo(thinPaths, outputPath) {
  const result = spawnSync('lipo', ['-create', ...thinPaths, '-output', outputPath], {
    encoding: 'utf8',
    stdio: 'pipe',
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(result.stderr || `lipo failed with exit code ${result.status ?? 1}`);
  }
}

function atomicCreateUniversal({
  thinBinaries,
  outputPath,
  createFat = createFatWithLipo,
}) {
  for (const binary of thinBinaries) {
    verifyThinMachO(binary.path, binary.target);
  }

  mkdirSync(path.dirname(outputPath), { recursive: true });
  const temporaryPath = `${outputPath}.tmp-${process.pid}-${Date.now()}`;
  try {
    createFat(thinBinaries.map((binary) => binary.path), temporaryPath);
    chmodSync(temporaryPath, 0o755);
    verifyUniversalMachO(temporaryPath);
    renameSync(temporaryPath, outputPath);
    verifyUniversalMachO(outputPath);
  } finally {
    rmSync(temporaryPath, { force: true });
  }
}

module.exports = {
  CPU_TYPES,
  atomicCreateUniversal,
  verifyExecutable,
  verifyThinMachO,
  verifyUniversalMachO,
};
