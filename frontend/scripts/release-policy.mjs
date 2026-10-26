#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import vm from 'node:vm';

import { parse as parseToml } from 'smol-toml';
import ts from 'typescript';
import { load as parseYaml } from 'js-yaml';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const frontendRoot = path.resolve(scriptDirectory, '..');
const repositoryRoot = path.resolve(frontendRoot, '..');

const TEXT_EXTENSIONS = new Set([
  '.cjs', '.css', '.env', '.html', '.js', '.json', '.jsx', '.md', '.mjs',
  '.plist', '.ps1', '.py', '.rs', '.sh', '.sql', '.toml', '.ts', '.tsx',
  '.txt', '.yaml', '.yml',
]);
const REVIEWED_CANDIDATE_VERIFIER_PATHS = [
  `/${'Users'}/runner/work/ort-artifacts/ort-artifacts/`,
  `/${'Users'}/runner/work/onnxruntime-build/onnxruntime-build/`,
  `/${'Users'}/runner/work/sherpa-onnx/sherpa-onnx/`,
  `/${'Users'}/Shared/ffmpeg/`,
];
const DIARIZATION_RESOURCE_SOURCE = 'resources/diarization/';
const DIARIZATION_RESOURCE_TARGET = 'diarization/';
const UPDATER_OWNED_FILES = [
  'frontend/src/services/updateService.ts',
  'frontend/src/hooks/useUpdateCheck.ts',
  'frontend/src/components/UpdateCheckProvider.tsx',
  'frontend/src/components/UpdateDialog.tsx',
  'frontend/src/components/UpdateNotification.tsx',
];
const UPDATER_LOCAL_MODULES = new Set(UPDATER_OWNED_FILES.map(
  (filePath) => filePath.replace(/\.(?:ts|tsx)$/, ''),
));

function finding(code, filePath, message, remediation) {
  return { code, path: filePath, message, remediation };
}

function globMatches(pattern, candidate) {
  const escaped = pattern.replace(/[.+^${}()|[\]\\]/g, '\\$&');
  const expression = `^${escaped.replaceAll('**', '.+').replaceAll('*', '[^/]+')}$`;
  return new RegExp(expression).test(candidate);
}

function resourceEntries(resources) {
  if (Array.isArray(resources)) {
    return resources.map((source) => [source, null]);
  }
  return Object.entries(resources ?? {});
}

function resourceSourceMatches(source, candidate) {
  return source.endsWith('/') ? candidate.startsWith(source) : globMatches(source, candidate);
}

function dependencyPackages(table, packages) {
  if (!table || typeof table !== 'object' || Array.isArray(table)) return;
  for (const [name, specification] of Object.entries(table)) {
    packages.add(name);
    if (specification && typeof specification === 'object'
      && typeof specification.package === 'string') {
      packages.add(specification.package);
    }
  }
}

function productionCargoDependencies(content) {
  const manifest = parseToml(content);
  const dependencies = new Set();
  dependencyPackages(manifest.dependencies, dependencies);
  for (const target of Object.values(manifest.target ?? {})) {
    dependencyPackages(target?.dependencies, dependencies);
  }
  return dependencies;
}

function cargoLockPackages(content) {
  const lock = parseToml(content);
  return new Set((lock.package ?? []).map(({ name }) => name).filter(Boolean));
}

function productionJavaScriptPath(filePath) {
  const normalized = filePath.replaceAll('\\', '/');
  return normalized.startsWith('frontend/src/')
    && /\.(?:[cm]?[jt]sx?)$/.test(normalized)
    && !normalized.startsWith('frontend/src/testing/')
    && !/(?:^|\/)tests?\//.test(normalized)
    && !/\.(?:test|spec)\.[cm]?[jt]sx?$/.test(normalized);
}

function javaScriptKind(filePath) {
  const extension = path.extname(filePath).toLowerCase();
  if (extension === '.js' || extension === '.mjs' || extension === '.cjs') return ts.ScriptKind.JS;
  if (extension === '.jsx') return ts.ScriptKind.JSX;
  if (extension === '.tsx') return ts.ScriptKind.TSX;
  return ts.ScriptKind.TS;
}

function javaScriptPolicyFacts(filePath, content) {
  const sourceFile = ts.createSourceFile(
    filePath,
    content,
    ts.ScriptTarget.Latest,
    true,
    javaScriptKind(filePath),
  );
  if (sourceFile.parseDiagnostics.length > 0) {
    const message = ts.flattenDiagnosticMessageText(sourceFile.parseDiagnostics[0].messageText, '\n');
    throw new SyntaxError(`Cannot parse ${filePath}: ${message}`);
  }
  const imports = [];
  const runtimeStrings = [];
  const stringValue = (node) => (
    ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node) ? node.text : null
  );

  const visit = (node) => {
    if (ts.isImportDeclaration(node)) {
      const specifier = stringValue(node.moduleSpecifier);
      if (specifier !== null) imports.push(specifier);
    } else if (ts.isImportEqualsDeclaration(node)
      && ts.isExternalModuleReference(node.moduleReference)) {
      const specifier = stringValue(node.moduleReference.expression);
      if (specifier !== null) imports.push(specifier);
    } else if (ts.isCallExpression(node) && node.expression.kind === ts.SyntaxKind.ImportKeyword) {
      const specifier = stringValue(node.arguments[0]);
      if (specifier !== null) imports.push(specifier);
    }

    const literal = stringValue(node);
    if (literal !== null) runtimeStrings.push(literal);
    if (ts.isTemplateExpression(node)) {
      runtimeStrings.push(node.head.text);
      for (const span of node.templateSpans) runtimeStrings.push(span.literal.text);
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return { imports, runtimeStrings };
}

function javaScriptInvokedCommands(filePath, content) {
  const sourceFile = ts.createSourceFile(
    filePath,
    content,
    ts.ScriptTarget.Latest,
    true,
    javaScriptKind(filePath),
  );
  if (sourceFile.parseDiagnostics.length > 0) {
    const message = ts.flattenDiagnosticMessageText(sourceFile.parseDiagnostics[0].messageText, '\n');
    throw new SyntaxError(`Cannot parse ${filePath}: ${message}`);
  }

  const bindingsByScope = new Map();
  const lexicalScope = (node) => {
    let current = node?.parent;
    while (current && !ts.isSourceFile(current)
      && !ts.isBlock(current)
      && !ts.isModuleBlock(current)
      && !ts.isCaseBlock(current)
      && !ts.isCatchClause(current)
      && !ts.isForStatement(current)
      && !ts.isForInStatement(current)
      && !ts.isForOfStatement(current)) {
      current = current.parent;
    }
    return current;
  };
  const register = (node) => {
    if (ts.isVariableDeclaration(node) && ts.isIdentifier(node.name) && node.initializer) {
      const scope = lexicalScope(node);
      const bindings = bindingsByScope.get(scope) ?? new Map();
      const declarations = bindings.get(node.name.text) ?? [];
      declarations.push(node);
      bindings.set(node.name.text, declarations);
      bindingsByScope.set(scope, bindings);
    }
    ts.forEachChild(node, register);
  };
  register(sourceFile);

  const unwrap = (node) => {
    let expression = node;
    while (expression && (ts.isParenthesizedExpression(expression)
      || ts.isAsExpression(expression)
      || ts.isTypeAssertionExpression(expression)
      || ts.isNonNullExpression(expression)
      || (ts.isSatisfiesExpression && ts.isSatisfiesExpression(expression)))) {
      expression = expression.expression;
    }
    return expression;
  };
  const staticString = (node, resolving = new Set()) => {
    const expression = unwrap(node);
    if (!expression) return null;
    if (ts.isStringLiteral(expression) || ts.isNoSubstitutionTemplateLiteral(expression)) {
      return expression.text;
    }
    if (ts.isIdentifier(expression)) {
      let scope = lexicalScope(expression);
      while (scope) {
        const declarations = bindingsByScope.get(scope)?.get(expression.text);
        if (declarations) {
          const declaration = declarations.findLast((candidate) => candidate.pos < expression.pos);
          if (!declaration || resolving.has(declaration)) return null;
          const next = new Set(resolving);
          next.add(declaration);
          return staticString(declaration.initializer, next);
        }
        scope = lexicalScope(scope);
      }
      return null;
    }
    if (ts.isBinaryExpression(expression)
      && expression.operatorToken.kind === ts.SyntaxKind.PlusToken) {
      const left = staticString(expression.left, resolving);
      const right = staticString(expression.right, resolving);
      return left === null || right === null ? null : left + right;
    }
    if (ts.isTemplateExpression(expression)) {
      let value = expression.head.text;
      for (const span of expression.templateSpans) {
        const part = staticString(span.expression, resolving);
        if (part === null) return null;
        value += part + span.literal.text;
      }
      return value;
    }
    return null;
  };

  const commands = new Set();
  const visit = (node) => {
    if (ts.isCallExpression(node)
      && ts.isIdentifier(node.expression)
      && node.expression.text === 'invoke') {
      const command = staticString(node.arguments[0]);
      if (command !== null) commands.add(command);
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return commands;
}

function resolvedLocalModule(filePath, specifier) {
  let resolved;
  if (specifier.startsWith('@/')) {
    resolved = `frontend/src/${specifier.slice(2)}`;
  } else if (specifier.startsWith('.')) {
    resolved = path.posix.normalize(path.posix.join(path.posix.dirname(filePath), specifier));
  } else {
    return null;
  }
  return resolved.replace(/\.(?:[cm]?[jt]sx?)$/, '');
}

function rustWithoutComments(content) {
  let result = '';
  let quote = null;
  let escaped = false;
  let lineComment = false;
  let blockDepth = 0;

  for (let index = 0; index < content.length; index += 1) {
    const character = content[index];
    const next = content[index + 1];

    if (lineComment) {
      if (character === '\n') {
        lineComment = false;
        result += character;
      }
      continue;
    }
    if (blockDepth > 0) {
      if (character === '/' && next === '*') {
        blockDepth += 1;
        index += 1;
      } else if (character === '*' && next === '/') {
        blockDepth -= 1;
        index += 1;
      } else if (character === '\n') {
        result += character;
      }
      continue;
    }
    if (quote) {
      result += character;
      if (character === '\\' && !escaped) {
        escaped = true;
        continue;
      }
      if (character === quote && !escaped) quote = null;
      escaped = false;
      continue;
    }
    if (character === '/' && next === '/') {
      lineComment = true;
      index += 1;
      continue;
    }
    if (character === '/' && next === '*') {
      blockDepth = 1;
      index += 1;
      continue;
    }
    if (character === '"' || (character === "'" && /^'(?:\\.|[^'\\\r\n])'/.test(content.slice(index)))) {
      quote = character;
    }
    result += character;
  }

  return result;
}

function rustStructuralSource(content) {
  const mask = (value) => value.replace(/[^\n]/g, ' ');
  return rustWithoutComments(content)
    .replace(/(?:br|r)(#{0,16})"[\s\S]*?"\1/g, mask)
    .replace(/b?"(?:\\[\s\S]|[^"\\])*"/g, mask)
    .replace(/b?'(?:\\[\s\S]|[^'\\\r\n])'/g, mask);
}

function rustMatchArmBodies(source, variant) {
  const bodies = [];
  const arm = new RegExp(`${variant}\\s*\\([^)]*\\)\\s*=>\\s*`, 'g');
  while (arm.exec(source) !== null) {
    let index = arm.lastIndex;
    if (source[index] === '{') {
      const start = index;
      let depth = 0;
      for (; index < source.length; index += 1) {
        if (source[index] === '{') depth += 1;
        if (source[index] === '}') {
          depth -= 1;
          if (depth === 0) {
            index += 1;
            break;
          }
        }
      }
      bodies.push(source.slice(start, index));
      arm.lastIndex = index;
      continue;
    }

    const start = index;
    let parentheses = 0;
    let brackets = 0;
    let braces = 0;
    for (; index < source.length; index += 1) {
      const character = source[index];
      if (character === '(') parentheses += 1;
      else if (character === ')') parentheses -= 1;
      else if (character === '[') brackets += 1;
      else if (character === ']') brackets -= 1;
      else if (character === '{') braces += 1;
      else if (character === '}') {
        if (parentheses === 0 && brackets === 0 && braces === 0) break;
        braces -= 1;
      } else if (character === ',' && parentheses === 0 && brackets === 0 && braces === 0) {
        break;
      }
    }
    bodies.push(source.slice(start, index));
    arm.lastIndex = index;
  }
  return bodies;
}

function rustNamedFunctionBodies(source) {
  const bodies = new Map();
  const functionStart = /\bfn\s+([A-Za-z_]\w*)\s*(?:<[^;{}]*>)?\s*\([^;{}]*\)\s*(?:->\s*[^;{]+)?\s*\{/g;
  let match;
  while ((match = functionStart.exec(source)) !== null) {
    const start = functionStart.lastIndex - 1;
    let index = start;
    let depth = 0;
    for (; index < source.length; index += 1) {
      if (source[index] === '{') depth += 1;
      if (source[index] === '}') {
        depth -= 1;
        if (depth === 0) {
          index += 1;
          break;
        }
      }
    }
    const namedBodies = bodies.get(match[1]) ?? [];
    namedBodies.push(source.slice(start, index));
    bodies.set(match[1], namedBodies);
    functionStart.lastIndex = index;
  }
  return bodies;
}

const LEGACY_EXECUTABLE = /^(?:python(?:3(?:\.\d+)?)?|uvicorn|fastapi|docker(?:-compose)?)$/i;
const LEGACY_EXECUTABLE_TOKEN = /(?:^|[\s;&|])(?:[^\s;&|]*\/)?(?:python(?:3(?:\.\d+)?)?|uvicorn|fastapi|docker(?:-compose)?)(?=$|[\s;&|])/i;
const LEGACY_SCRIPT_EXECUTABLE = /\.py[cowz]?$/i;
const LEGACY_SCRIPT_TOKEN = /(?:^|[\s;&|])["']?[^\s;&|"']*\.py[cowz]?["']?(?=$|[\s;&|])/i;

function executableBasename(value) {
  const executable = String(value).trim().split(/\s+/, 1)[0].replaceAll('\\', '/');
  return executable.split('/').at(-1)?.toLowerCase() ?? '';
}

function commandStringsLaunchLegacy(strings, shellCommand = false) {
  if (strings.length === 0) return false;
  const executable = executableBasename(strings[0]);
  if (LEGACY_EXECUTABLE.test(executable) || LEGACY_SCRIPT_EXECUTABLE.test(executable)) return true;
  const remaining = strings.slice(1).join(' ');
  if (executable === 'env') {
    return LEGACY_EXECUTABLE_TOKEN.test(remaining) || LEGACY_SCRIPT_TOKEN.test(remaining);
  }
  if (['sh', 'bash', 'zsh'].includes(executable)) {
    const shellText = remaining || strings[0];
    return LEGACY_EXECUTABLE_TOKEN.test(shellText) || LEGACY_SCRIPT_TOKEN.test(shellText);
  }
  return shellCommand
    && (LEGACY_EXECUTABLE_TOKEN.test(strings[0]) || LEGACY_SCRIPT_TOKEN.test(strings[0]));
}

function javaScriptLaunchesLegacyRuntime(filePath, content) {
  const sourceFile = ts.createSourceFile(
    filePath,
    content,
    ts.ScriptTarget.Latest,
    true,
    javaScriptKind(filePath),
  );
  if (sourceFile.parseDiagnostics.length > 0) {
    const message = ts.flattenDiagnosticMessageText(sourceFile.parseDiagnostics[0].messageText, '\n');
    throw new SyntaxError(`Cannot parse ${filePath}: ${message}`);
  }

  const processApis = new Set([
    'spawn',
    'spawnSync',
    'exec',
    'execSync',
    'execFile',
    'execFileSync',
  ]);
  const identifierApis = new Map();
  const processNamespaces = new Set();
  const stringValue = (node) => (
    ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node) ? node.text : null
  );
  const isChildProcessModule = (node) => {
    const value = node ? stringValue(node) : null;
    return value === 'child_process' || value === 'node:child_process';
  };
  const isChildProcessRequire = (node) => (
    node
    && ts.isCallExpression(node)
    && ts.isIdentifier(node.expression)
    && node.expression.text === 'require'
    && isChildProcessModule(node.arguments[0])
  );
  const unwrapExpression = (node) => {
    let expression = node;
    while (expression && (ts.isAwaitExpression(expression)
      || ts.isParenthesizedExpression(expression))) {
      expression = expression.expression;
    }
    return expression;
  };
  const isChildProcessDynamicImport = (node) => {
    const expression = unwrapExpression(node);
    return expression
      && ts.isCallExpression(expression)
      && expression.expression.kind === ts.SyntaxKind.ImportKeyword
      && isChildProcessModule(expression.arguments[0]);
  };
  const isChildProcessLoader = (node) => (
    isChildProcessRequire(node) || isChildProcessDynamicImport(node)
  );
  const registerSymbols = (node) => {
    if (ts.isImportDeclaration(node) && isChildProcessModule(node.moduleSpecifier)) {
      const clause = node.importClause;
      if (clause?.name) processNamespaces.add(clause.name.text);
      if (clause?.namedBindings && ts.isNamespaceImport(clause.namedBindings)) {
        processNamespaces.add(clause.namedBindings.name.text);
      }
      if (clause?.namedBindings && ts.isNamedImports(clause.namedBindings)) {
        for (const element of clause.namedBindings.elements) {
          const imported = element.propertyName?.text ?? element.name.text;
          if (processApis.has(imported)) identifierApis.set(element.name.text, imported);
        }
      }
    }
    if (ts.isImportEqualsDeclaration(node)
      && ts.isExternalModuleReference(node.moduleReference)
      && isChildProcessModule(node.moduleReference.expression)) {
      processNamespaces.add(node.name.text);
    }
    ts.forEachChild(node, registerSymbols);
  };
  registerSymbols(sourceFile);

  const bindingContainsName = (name, expected) => {
    if (ts.isIdentifier(name)) return name.text === expected;
    if (ts.isObjectBindingPattern(name) || ts.isArrayBindingPattern(name)) {
      return name.elements.some((element) => (
        ts.isBindingElement(element) && bindingContainsName(element.name, expected)
      ));
    }
    return false;
  };
  const localBinding = Object.freeze({ kind: 'local', definitelyTruthy: false });
  const localValueBinding = (expression) => ({
    kind: 'local',
    definitelyTruthy: Boolean(expression && (
      ts.isArrowFunction(expression)
      || ts.isFunctionExpression(expression)
      || ts.isClassExpression(expression)
      || ts.isObjectLiteralExpression(expression)
      || ts.isArrayLiteralExpression(expression)
      || ts.isNewExpression(expression)
      || expression.kind === ts.SyntaxKind.TrueKeyword
      || ((ts.isStringLiteral(expression) || ts.isNoSubstitutionTemplateLiteral(expression))
        && expression.text.length > 0)
    )),
  });
  const namespaceBinding = Object.freeze({ kind: 'namespace' });
  const apiBinding = (api) => ({ kind: 'api', api });
  const processBiasedBinding = (...bindings) => (
    bindings.find((binding) => binding?.kind === 'api')
    ?? bindings.find((binding) => binding?.kind === 'namespace')
    ?? localBinding
  );
  const importedBinding = (name) => {
    if (identifierApis.has(name)) return apiBinding(identifierApis.get(name));
    if (processNamespaces.has(name)) return namespaceBinding;
    return null;
  };
  const dynamicCallbackBinding = (parameter, name) => {
    const callback = parameter.parent;
    const call = callback?.parent;
    if (!(callback && (ts.isArrowFunction(callback) || ts.isFunctionExpression(callback)))
      || !ts.isCallExpression(call)
      || !ts.isPropertyAccessExpression(call.expression)
      || call.expression.name.text !== 'then'
      || !isChildProcessDynamicImport(call.expression.expression)) return localBinding;
    if (ts.isIdentifier(parameter.name)) {
      return parameter.name.text === name ? namespaceBinding : localBinding;
    }
    if (!ts.isObjectBindingPattern(parameter.name)) return localBinding;
    const element = parameter.name.elements.find((candidate) => (
      candidate.name.getText(sourceFile) === name
    ));
    if (!element) return localBinding;
    const imported = element.propertyName?.getText(sourceFile) ?? element.name.getText(sourceFile);
    return processApis.has(imported) ? apiBinding(imported) : localBinding;
  };
  let bindingForIdentifier;
  let initializerBinding;
  const memberApi = (expression) => {
    if (!expression) return null;
    const isProperty = ts.isPropertyAccessExpression(expression);
    const isElement = ts.isElementAccessExpression(expression);
    if (!isProperty && !isElement) return null;
    const receiver = unwrapExpression(expression.expression);
    const receiverBinding = isChildProcessLoader(receiver)
      ? namespaceBinding
      : (ts.isIdentifier(receiver) ? bindingForIdentifier(receiver) : null);
    if (receiverBinding?.kind !== 'namespace') return null;
    const name = isProperty ? expression.name.text : stringValue(expression.argumentExpression);
    return processApis.has(name) ? name : null;
  };
  initializerBinding = (initializer) => {
    if (!initializer) return localBinding;
    const expression = unwrapExpression(initializer);
    if (isChildProcessLoader(expression)) return namespaceBinding;
    if (ts.isIdentifier(expression)) return bindingForIdentifier(expression) ?? localBinding;
    if (ts.isConditionalExpression(expression)) {
      return processBiasedBinding(
        initializerBinding(expression.whenTrue),
        initializerBinding(expression.whenFalse),
      );
    }
    if (ts.isBinaryExpression(expression)
      && [
        ts.SyntaxKind.BarBarToken,
        ts.SyntaxKind.AmpersandAmpersandToken,
        ts.SyntaxKind.QuestionQuestionToken,
      ].includes(expression.operatorToken.kind)) {
      return processBiasedBinding(
        initializerBinding(expression.left),
        initializerBinding(expression.right),
      );
    }
    if (ts.isCallExpression(expression)
      && ts.isPropertyAccessExpression(expression.expression)
      && expression.expression.name.text === 'bind') {
      const target = unwrapExpression(expression.expression.expression);
      if (ts.isIdentifier(target)) {
        const binding = bindingForIdentifier(target);
        if (binding?.kind === 'api') return binding;
      }
      const api = memberApi(target);
      if (api) return apiBinding(api);
    }
    const api = memberApi(expression);
    return api ? apiBinding(api) : localValueBinding(expression);
  };
  const declarationBinding = (declaration, name) => {
    if (ts.isIdentifier(declaration.name)) {
      const initializer = unwrapExpression(declaration.initializer);
      if (initializer && ts.isIdentifier(initializer) && initializer.text === name) {
        return localBinding;
      }
      return initializerBinding(declaration.initializer);
    }
    if (!ts.isObjectBindingPattern(declaration.name)) return localBinding;
    const element = declaration.name.elements.find((candidate) => (
      candidate.name.getText(sourceFile) === name
    ));
    if (!element || initializerBinding(declaration.initializer).kind !== 'namespace') {
      return localBinding;
    }
    const imported = element.propertyName?.getText(sourceFile) ?? element.name.getText(sourceFile);
    return processApis.has(imported) ? apiBinding(imported) : localBinding;
  };
  const assignmentBinding = (left, right, name) => {
    const target = unwrapExpression(left);
    if (ts.isIdentifier(target)) {
      return target.text === name ? initializerBinding(right) : null;
    }
    if (ts.isObjectLiteralExpression(target)) {
      if (initializerBinding(right).kind !== 'namespace') return localBinding;
      for (const property of target.properties) {
        if (ts.isPropertyAssignment(property)
          && ts.isIdentifier(property.initializer)
          && property.initializer.text === name) {
          const imported = property.name.getText(sourceFile);
          return processApis.has(imported) ? apiBinding(imported) : localBinding;
        }
        if (ts.isShorthandPropertyAssignment(property) && property.name.text === name) {
          return processApis.has(name) ? apiBinding(name) : localBinding;
        }
      }
      return null;
    }
    if (ts.isArrayLiteralExpression(target)) {
      const index = target.elements.findIndex((element) => (
        ts.isIdentifier(element) && element.text === name
      ));
      if (index < 0) return null;
      const values = unwrapExpression(right);
      if (!ts.isArrayLiteralExpression(values) || index >= values.elements.length) {
        return localBinding;
      }
      return initializerBinding(values.elements[index]);
    }
    return null;
  };
  const latestBindingState = (identifier) => {
    let state = null;
    let statePosition = Number.NEGATIVE_INFINITY;
    const update = (position, binding) => {
      if (position <= identifier.pos && position >= statePosition) {
        state = binding;
        statePosition = position;
      }
    };
    const scanAssignments = (node) => {
      if (node.pos >= identifier.pos || ts.isClassDeclaration(node)
        || ts.isVariableDeclaration(node)) return;
      if (ts.isFunctionLike(node)) {
        const call = node.parent;
        const dynamicCallback = ts.isCallExpression(call)
          && ts.isPropertyAccessExpression(call.expression)
          && call.expression.name.text === 'then'
          && isChildProcessDynamicImport(call.expression.expression);
        if (dynamicCallback) ts.forEachChild(node, scanAssignments);
        return;
      }
      if (ts.isBinaryExpression(node)
        && [
          ts.SyntaxKind.EqualsToken,
          ts.SyntaxKind.BarBarEqualsToken,
          ts.SyntaxKind.AmpersandAmpersandEqualsToken,
          ts.SyntaxKind.QuestionQuestionEqualsToken,
        ].includes(node.operatorToken.kind)) {
        const binding = assignmentBinding(node.left, node.right, identifier.text);
        if (binding) {
          const logicalAssignment = node.operatorToken.kind !== ts.SyntaxKind.EqualsToken;
          const truthyOrAssignmentCannotRun = node.operatorToken.kind
            === ts.SyntaxKind.BarBarEqualsToken
            && state?.kind === 'local'
            && state.definitelyTruthy;
          if (!truthyOrAssignmentCannotRun
            && (!logicalAssignment || binding.kind !== 'local')) update(node.pos, binding);
        }
      }
      ts.forEachChild(node, scanAssignments);
    };
    const visitContainingScopes = (node) => {
      if (node.pos > identifier.pos || node.end < identifier.end) return;
      if (ts.isFunctionLike(node)) {
        for (const parameter of node.parameters) {
          if (bindingContainsName(parameter.name, identifier.text)) {
            update(node.body?.pos ?? node.pos, dynamicCallbackBinding(parameter, identifier.text));
          }
        }
      }
      if (ts.isCatchClause(node)
        && node.variableDeclaration
        && bindingContainsName(node.variableDeclaration.name, identifier.text)) {
        update(node.block.pos, localBinding);
      }
      if ((ts.isForInStatement(node) || ts.isForOfStatement(node))
        && ts.isVariableDeclarationList(node.initializer)) {
        for (const declaration of node.initializer.declarations) {
          if (bindingContainsName(declaration.name, identifier.text)) {
            update(declaration.pos, declarationBinding(declaration, identifier.text));
          }
        }
      } else if ((ts.isForInStatement(node) || ts.isForOfStatement(node))
        && ts.isIdentifier(node.initializer)
        && node.initializer.text === identifier.text) {
        update(node.initializer.pos, localBinding);
      }
      if (ts.isForStatement(node) && node.initializer
        && ts.isVariableDeclarationList(node.initializer)) {
        for (const declaration of node.initializer.declarations) {
          if (bindingContainsName(declaration.name, identifier.text)) {
            update(declaration.pos, declarationBinding(declaration, identifier.text));
          }
        }
      }
      if (ts.isFunctionLike(node) && node.body && !ts.isBlock(node.body)) {
        scanAssignments(node.body);
      }
      if (ts.isSourceFile(node) || ts.isBlock(node)) {
        for (const statement of node.statements) {
          if (statement.pos >= identifier.pos) continue;
          if (ts.isVariableStatement(statement)) {
            for (const declaration of statement.declarationList.declarations) {
              if (bindingContainsName(declaration.name, identifier.text)) {
                update(declaration.pos, declarationBinding(declaration, identifier.text));
              }
            }
          }
          if ((ts.isFunctionDeclaration(statement) || ts.isClassDeclaration(statement))
            && statement.name?.text === identifier.text) {
            update(statement.pos, localBinding);
          }
          scanAssignments(statement);
        }
      }
      ts.forEachChild(node, visitContainingScopes);
    };
    visitContainingScopes(sourceFile);
    return state;
  };
  bindingForIdentifier = (identifier) => (
    latestBindingState(identifier) ?? importedBinding(identifier.text)
  );
  const identifierIsShadowed = (identifier) => (
    bindingForIdentifier(identifier)?.kind === 'local'
  );
  const processApi = (expression) => {
    const unwrapped = unwrapExpression(expression);
    if (ts.isIdentifier(unwrapped)) {
      const binding = bindingForIdentifier(unwrapped);
      return binding?.kind === 'api' ? binding.api : null;
    }
    return memberApi(unwrapped);
  };

  let launches = false;
  const collectStrings = (node, values) => {
    if (!node) return;
    if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) {
      values.push(node.text);
      return;
    }
    if (ts.isArrayLiteralExpression(node)) {
      for (const element of node.elements) collectStrings(element, values);
    }
  };
  const visit = (node) => {
    if (launches) return;
    if (ts.isCallExpression(node)) {
      if (ts.isPropertyAccessExpression(node.expression)
        && node.expression.name.text === 'then'
        && isChildProcessDynamicImport(node.expression.expression)) {
        const callback = node.arguments[0];
        const parameter = callback
          && (ts.isArrowFunction(callback) || ts.isFunctionExpression(callback))
          ? callback.parameters[0]
          : null;
        if (parameter) {
          const callbackApis = new Map();
          const callbackNamespaces = new Set();
          if (ts.isIdentifier(parameter.name)) {
            callbackNamespaces.add(parameter.name.text);
          } else if (ts.isObjectBindingPattern(parameter.name)) {
            for (const element of parameter.name.elements) {
              const imported = element.propertyName?.getText(sourceFile)
                ?? element.name.getText(sourceFile);
              const local = element.name.getText(sourceFile);
              if (processApis.has(imported)) callbackApis.set(local, imported);
            }
          }
          const visitCallback = (callbackNode) => {
            if (launches) return;
            if (ts.isCallExpression(callbackNode)) {
              let api = null;
              if (ts.isIdentifier(callbackNode.expression)
                && callbackApis.has(callbackNode.expression.text)
                && !identifierIsShadowed(callbackNode.expression)) {
                api = callbackApis.get(callbackNode.expression.text);
              } else if (ts.isPropertyAccessExpression(callbackNode.expression)
                && ts.isIdentifier(callbackNode.expression.expression)
                && callbackNamespaces.has(callbackNode.expression.expression.text)
                && !identifierIsShadowed(callbackNode.expression.expression)
                && processApis.has(callbackNode.expression.name.text)) {
                api = callbackNode.expression.name.text;
              }
              if (api) {
              const strings = [];
              collectStrings(callbackNode.arguments[0], strings);
              if (strings.length === 0) launches = true;
              else {
                for (const argument of callbackNode.arguments.slice(1)) {
                  collectStrings(argument, strings);
                }
                launches = commandStringsLaunchLegacy(
                  strings,
                  api === 'exec' || api === 'execSync',
                );
              }
              }
            }
            ts.forEachChild(callbackNode, visitCallback);
          };
          visitCallback(callback.body);
        }
      }
      const name = processApi(node.expression);
      if (name) {
        const strings = [];
        collectStrings(node.arguments[0], strings);
        if (strings.length === 0) {
          launches = true;
        } else {
          for (const argument of node.arguments.slice(1)) collectStrings(argument, strings);
          launches = commandStringsLaunchLegacy(
            strings,
            name === 'exec' || name === 'execSync',
          );
        }
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return launches;
}

function rustLaunchesLegacyRuntime(filePath, content) {
  const source = rustWithoutComments(content);
  const constants = new Map();
  const constantPattern = /\b(?:const|static)\s+([A-Za-z_]\w*)\s*(?::[^=;]+)?=\s*"((?:\\.|[^"\\])*)"\s*;/g;
  for (const match of source.matchAll(constantPattern)) {
    try {
      constants.set(match[1], JSON.parse(`"${match[2]}"`));
    } catch {
      constants.set(match[1], match[2]);
    }
  }

  const matchingParenthesis = (openIndex) => {
    let depth = 0;
    let quote = null;
    let escaped = false;
    for (let index = openIndex; index < source.length; index += 1) {
      const character = source[index];
      if (quote) {
        if (character === '\\' && !escaped) {
          escaped = true;
          continue;
        }
        if (character === quote && !escaped) quote = null;
        escaped = false;
        continue;
      }
      if (character === '"' || character === "'") {
        quote = character;
      } else if (character === '(') {
        depth += 1;
      } else if (character === ')' && --depth === 0) {
        return index;
      }
    }
    return -1;
  };
  const statementEnd = (startIndex) => {
    let quote = null;
    let escaped = false;
    let depth = 0;
    for (let index = startIndex; index < source.length; index += 1) {
      const character = source[index];
      if (quote) {
        if (character === '\\' && !escaped) {
          escaped = true;
          continue;
        }
        if (character === quote && !escaped) quote = null;
        escaped = false;
        continue;
      }
      if (character === '"' || character === "'") quote = character;
      else if ('([{'.includes(character)) depth += 1;
      else if (')]}'.includes(character)) depth = Math.max(0, depth - 1);
      else if (character === ';' && depth === 0) return index + 1;
    }
    return source.length;
  };
  const expressionString = (expression) => {
    const trimmed = expression.trim();
    const literal = trimmed.match(/^"((?:\\.|[^"\\])*)"$/s);
    if (literal) {
      try {
        return JSON.parse(`"${literal[1]}"`);
      } catch {
        return literal[1];
      }
    }
    const raw = trimmed.match(/^r(#{0,255})"([\s\S]*)"\1$/);
    if (raw) return raw[2];
    const identifier = trimmed.match(/^&?\s*([A-Za-z_]\w*)$/);
    return identifier ? constants.get(identifier[1]) ?? null : null;
  };
  const splitTopLevel = (text) => {
    const parts = [];
    let start = 0;
    let depth = 0;
    let quote = null;
    let escaped = false;
    for (let index = 0; index < text.length; index += 1) {
      const character = text[index];
      if (quote) {
        if (character === '\\' && !escaped) {
          escaped = true;
          continue;
        }
        if (character === quote && !escaped) quote = null;
        escaped = false;
        continue;
      }
      if (character === '"' || character === "'") quote = character;
      else if ('([{'.includes(character)) depth += 1;
      else if (')]}'.includes(character)) depth = Math.max(0, depth - 1);
      else if (character === ',' && depth === 0) {
        parts.push(text.slice(start, index));
        start = index + 1;
      }
    }
    parts.push(text.slice(start));
    return parts.map((part) => part.trim()).filter(Boolean);
  };
  const argumentValues = (text, baseOffset) => {
    const values = [];
    let unknown = false;
    const argumentPattern = /\.(arg|args)\s*\(/g;
    for (const match of text.matchAll(argumentPattern)) {
      const open = baseOffset + match.index + match[0].lastIndexOf('(');
      const close = matchingParenthesis(open);
      if (close < 0) {
        unknown = true;
        continue;
      }
      const expression = source.slice(open + 1, close);
      const expressions = match[1] === 'args'
        ? splitTopLevel(
          expression.match(/^\s*&?\s*(?:vec!\s*)?\[([\s\S]*)\]\s*$/)?.[1] ?? '',
        )
        : [expression];
      if (match[1] === 'args' && expressions.length === 0 && expression.trim() !== '[]') {
        unknown = true;
      }
      for (const item of expressions) {
        const value = expressionString(item);
        if (value === null) unknown = true;
        else values.push(value);
      }
    }
    return { values, unknown };
  };

  const enclosingBraceStarts = (beforeIndex) => {
    const stack = [];
    let quote = false;
    let escaped = false;
    let rawHashes = null;
    for (let index = 0; index < beforeIndex; index += 1) {
      if (rawHashes !== null) {
        const terminator = `"${'#'.repeat(rawHashes)}`;
        if (source.startsWith(terminator, index)) {
          index += terminator.length - 1;
          rawHashes = null;
        }
        continue;
      }
      if (quote) {
        if (source[index] === '\\' && !escaped) {
          escaped = true;
          continue;
        }
        if (source[index] === '"' && !escaped) quote = false;
        escaped = false;
        continue;
      }
      const raw = source.slice(index).match(/^r(#{0,255})"/);
      if (raw) {
        rawHashes = raw[1].length;
        index += raw[0].length - 1;
      } else if (source[index] === '"') {
        quote = true;
      } else if (source[index] === '{') {
        stack.push(index);
      } else if (source[index] === '}') {
        stack.pop();
      }
    }
    return stack;
  };
  const commandBindings = [];
  const addCommandBinding = (name, kind, index) => {
    commandBindings.push({
      name,
      kind,
      scope: enclosingBraceStarts(index).at(-1) ?? -1,
    });
  };
  const parseStandardProcessItem = (item, index) => {
    if (item === 'process') {
      addCommandBinding('process', 'process-namespace', index);
      return;
    }
    let match = item.match(/^processas([A-Za-z_]\w*)$/);
    if (match) {
      addCommandBinding(match[1], 'process-namespace', index);
      return;
    }
    if (item === 'process::*') {
      addCommandBinding('Command', 'process-command', index);
      return;
    }
    match = item.match(/^process::Command(?:as([A-Za-z_]\w*))?$/);
    if (match) {
      addCommandBinding(match[1] ?? 'Command', 'process-command', index);
      return;
    }
    if (item === 'process::self') {
      addCommandBinding('process', 'process-namespace', index);
      return;
    }
    match = item.match(/^process::selfas([A-Za-z_]\w*)$/);
    if (match) {
      addCommandBinding(match[1], 'process-namespace', index);
    }
  };
  const expandUseTree = (tree) => {
    const normalized = tree.replace(/^::/, '');
    const rootGroup = normalized.match(/^\{([\s\S]*)\}$/);
    if (rootGroup) {
      return splitTopLevel(rootGroup[1]).flatMap((item) => expandUseTree(item));
    }
    const nestedGroup = normalized.match(/^([\s\S]*?)::\{([\s\S]*)\}$/);
    if (!nestedGroup) return [normalized];
    return splitTopLevel(nestedGroup[2]).flatMap((item) => (
      expandUseTree(`${nestedGroup[1]}::${item}`)
    ));
  };
  for (const useMatch of source.matchAll(/\buse\s+([^;]+);/g)) {
    const normalized = useMatch[1].replace(/\s+/g, '');
    for (const item of expandUseTree(normalized)) {
      const standard = item.match(/^(?:std|tokio)::([\s\S]+)$/);
      if (standard) {
        parseStandardProcessItem(standard[1], useMatch.index);
        continue;
      }
      let local = item.match(/(?:^|::)Command(?:as([A-Za-z_]\w*))?$/);
      if (local) addCommandBinding(local[1] ?? 'Command', 'local-command', useMatch.index);
      local = item.match(/(?:^|::)([A-Za-z_]\w*)(?:as([A-Za-z_]\w*))?$/);
      if (local) addCommandBinding(local[2] ?? local[1], 'local-namespace', useMatch.index);
    }
  }
  for (const match of source.matchAll(/\btype\s+([A-Za-z_]\w*)\s*=\s*(?:std|tokio)\s*::\s*process\s*::\s*Command\s*;/g)) {
    addCommandBinding(match[1], 'process-command', match.index);
  }
  for (const match of source.matchAll(/\b(struct|enum|union|trait|type)\s+([A-Za-z_]\w*)\b/g)) {
    const standardAlias = match[1] === 'type'
      && /^type\s+[A-Za-z_]\w*\s*=\s*(?:std|tokio)\s*::\s*process\s*::\s*Command\b/
        .test(source.slice(match.index));
    if (!standardAlias) addCommandBinding(match[2], 'local-command', match.index);
  }
  for (const match of source.matchAll(/\bmod\s+([A-Za-z_]\w*)\b/g)) {
    addCommandBinding(match[1], 'local-namespace', match.index);
  }
  const visibleCommandBinding = (name, beforeIndex) => {
    const scopes = [-1, ...enclosingBraceStarts(beforeIndex)];
    const visible = commandBindings.filter((binding) => (
      binding.name === name && scopes.includes(binding.scope)
    ));
    if (visible.length === 0) return null;
    const deepest = Math.max(...visible.map((binding) => scopes.indexOf(binding.scope)));
    const nearest = visible.filter((binding) => scopes.indexOf(binding.scope) === deepest);
    return nearest.find((binding) => binding.kind.startsWith('local-')) ?? nearest.at(-1);
  };
  const resolvesToProcessCommand = (commandType, beforeIndex) => {
    const normalized = commandType.replace(/\s+/g, '');
    if (/^(?:std|tokio)::process::Command$/.test(normalized)) return true;
    const parts = normalized.split('::');
    if (parts.length === 1) {
      return visibleCommandBinding(parts[0], beforeIndex)?.kind === 'process-command';
    }
    if (parts.length === 2 && parts[1] === 'Command') {
      return visibleCommandBinding(parts[0], beforeIndex)?.kind === 'process-namespace';
    }
    return false;
  };
  const reviewedFfmpegExpressions = new Map([
    ['frontend/src-tauri/src/audio/decoder.rs', new Set(['&ffmpeg_path'])],
    ['frontend/src-tauri/src/audio/encode.rs', new Set(['ffmpeg_path'])],
    [
      'frontend/src-tauri/src/audio/incremental_saver.rs',
      new Set(['&ffmpeg', 'ffmpeg', 'ffmpeg_path', 'find_ffmpeg_path().unwrap()']),
    ],
  ]);
  const isReviewedFfmpegExpression = (expression, beforeIndex) => {
    const normalized = expression.replace(/\s+/g, '');
    if (!(reviewedFfmpegExpressions.get(filePath)?.has(normalized) ?? false)) return false;
    if (normalized === 'find_ffmpeg_path().unwrap()') return true;
    const identifier = normalized.match(/^&?(ffmpeg(?:_path)?)$/)?.[1];
    if (!identifier) return false;
    const assignmentPattern = new RegExp(`\\blet\\s+${identifier}\\s*=\\s*([^;]+);`, 'g');
    const assignments = [...source.slice(0, beforeIndex).matchAll(assignmentPattern)];
    if (assignments.length === 0) return false;
    const latest = assignments.at(-1);
    const rightHandSide = latest[1].replace(/\s+/g, '');
    if (!/^find_ffmpeg_path\(\)(?:\.unwrap\(\)|\.ok_or_else\([\s\S]*\)\?)$/.test(rightHandSide)) {
      return false;
    }
    const assignmentEnd = latest.index + latest[0].length;
    const afterAssignment = source.slice(assignmentEnd, beforeIndex);
    if (new RegExp(`\\b${identifier}\\s*=`).test(afterAssignment)) return false;
    const closureRegion = afterAssignment.slice(
      Math.max(afterAssignment.lastIndexOf(';'), afterAssignment.lastIndexOf('}')) + 1,
    );
    for (const closure of closureRegion.matchAll(/\|([^|]*)\|/g)) {
      if (new RegExp(`\\b${identifier}\\b`).test(closure[1])) return false;
    }

    const functionPattern = /\bfn\s+[A-Za-z_]\w*\s*\(([^)]*)\)[^{]*\{/g;
    for (const functionMatch of source.matchAll(functionPattern)) {
      if (functionMatch.index >= beforeIndex
        || !new RegExp(`\\b${identifier}\\b`).test(functionMatch[1])) continue;
      const open = functionMatch.index + functionMatch[0].lastIndexOf('{');
      let depth = 0;
      for (let index = open; index < source.length; index += 1) {
        if (source[index] === '{') depth += 1;
        else if (source[index] === '}' && --depth === 0) {
          if (beforeIndex < index) return false;
          break;
        }
      }
    }
    return true;
  };

  const startPattern = /\b((?:[A-Za-z_]\w*\s*::\s*)*[A-Za-z_]\w*)\s*::\s*(?:new|new_sidecar)\s*\(/g;
  for (const match of source.matchAll(startPattern)) {
    if (!resolvesToProcessCommand(match[1], match.index)) continue;
    const open = match.index + match[0].lastIndexOf('(');
    const close = matchingParenthesis(open);
    if (close < 0) return true;
    const executableExpression = source.slice(open + 1, close);
    const executable = expressionString(executableExpression);
    if (executable === null) {
      if (!isReviewedFfmpegExpression(executableExpression, match.index)) return true;
      continue;
    }

    const start = Math.max(
      source.lastIndexOf(';', match.index - 1),
      source.lastIndexOf('{', match.index - 1),
      source.lastIndexOf('}', match.index - 1),
    ) + 1;
    const assignment = source.slice(start, match.index)
      .match(/\blet\s+(?:mut\s+)?([A-Za-z_]\w*)\s*=\s*$/);
    const initial = source.slice(close + 1, statementEnd(close + 1));
    const initialArguments = argumentValues(initial, close + 1);
    const strings = [executable, ...initialArguments.values];
    let unknownWrapperArgument = initialArguments.unknown;
    if (assignment) {
      const variablePattern = new RegExp(`\\b${assignment[1]}\\s*\\.(?:arg|args)\\s*\\(`, 'g');
      for (const variableMatch of source.slice(close + 1).matchAll(variablePattern)) {
        const statementStart = close + 1 + variableMatch.index;
        const parsed = argumentValues(
          source.slice(statementStart, statementEnd(statementStart)),
          statementStart,
        );
        strings.push(...parsed.values);
        unknownWrapperArgument ||= parsed.unknown;
      }
    }
    if (commandStringsLaunchLegacy(strings)) return true;
    if (['env', 'sh', 'bash', 'zsh'].includes(executableBasename(executable))
      && unknownWrapperArgument) return true;
  }

  return false;
}

function executableWorkflowCommands(filePath, content) {
  if (filePath.endsWith('.env.example')) {
    return content.split(/\r?\n/)
      .map((line) => line.match(/^\s*[A-Za-z_][A-Za-z0-9_]*\s*=\s*(.*?)\s*$/)?.[1])
      .filter(Boolean)
      .map((command) => ({ kind: 'environment', command }));
  }
  if (!/\.ya?ml$/i.test(filePath)) return [];
  const commands = [];
  const visit = (value) => {
    if (Array.isArray(value)) {
      for (const item of value) visit(item);
    } else if (value && typeof value === 'object') {
      for (const [key, nested] of Object.entries(value)) {
        if ((key === 'run' || key === 'shell') && typeof nested === 'string') {
          commands.push({ kind: key, command: nested });
        }
        else visit(nested);
      }
    }
  };
  visit(parseYaml(content));
  return commands;
}

function embeddedPermissions(tauriConfig) {
  const capabilities = tauriConfig.app?.security?.capabilities;
  if (!Array.isArray(capabilities)) return [];
  return capabilities.flatMap((capability) => (
    capability && typeof capability === 'object' && Array.isArray(capability.permissions)
      ? capability.permissions
      : []
  ));
}

function capabilityPermissions(content) {
  const capability = typeof content === 'string' ? JSON.parse(content) : content;
  return Array.isArray(capability?.permissions) ? capability.permissions : [];
}

function permissionIdentifier(permission) {
  if (typeof permission === 'string') return permission;
  return permission && typeof permission === 'object' && typeof permission.identifier === 'string'
    ? permission.identifier
    : null;
}

function workflowPolicyFacts(filePath, content) {
  const keys = new Set();
  const strings = [];
  if (filePath.endsWith('.env.example')) {
    for (const line of content.split(/\r?\n/)) {
      const match = line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*?)\s*$/);
      if (!match) continue;
      keys.add(match[1]);
      strings.push(match[2]);
    }
  } else if (!/\.ya?ml$/.test(filePath)) {
    strings.push(content);
    for (const key of ['TAURI_SIGNING_PRIVATE_KEY', 'TAURI_SIGNING_PRIVATE_KEY_PASSWORD']) {
      if (content.includes(key)) keys.add(key);
    }
  } else {
    const seen = new WeakSet();
    const visit = (value) => {
      if (typeof value === 'string') {
        strings.push(value);
      } else if (Array.isArray(value)) {
        for (const item of value) visit(item);
      } else if (value && typeof value === 'object' && !seen.has(value)) {
        seen.add(value);
        for (const [key, nestedValue] of Object.entries(value)) {
          keys.add(key);
          visit(nestedValue);
        }
      }
    };
    visit(parseYaml(content));
  }
  return { keys, searchableText: [...keys, ...strings].join('\n') };
}

function manifestHasDependency(packageJson, dependency) {
  for (const field of ['dependencies', 'optionalDependencies', 'devDependencies', 'peerDependencies']) {
    const dependencies = packageJson[field] ?? {};
    if (Object.hasOwn(dependencies, dependency)) return true;
    if (Object.values(dependencies).some((version) => (
      typeof version === 'string'
      && (version === `npm:${dependency}` || version.startsWith(`npm:${dependency}@`))
    ))) return true;
  }
  for (const field of ['bundledDependencies', 'bundleDependencies']) {
    const dependencies = packageJson[field];
    if (Array.isArray(dependencies) && dependencies.includes(dependency)) return true;
    if (dependencies && !Array.isArray(dependencies) && Object.hasOwn(dependencies, dependency)) return true;
  }
  return false;
}

function cargoDependencyFeatures(specification) {
  return new Set(
    specification && typeof specification === 'object' && Array.isArray(specification.features)
      ? specification.features
      : [],
  );
}

function normalizedCargoTarget(target) {
  return target.replaceAll(/\s+/g, '').toLowerCase();
}

function universalHelperCargoRouting(source) {
  const sourceFile = ts.createSourceFile(
    'frontend/scripts/build-universal-macos.js',
    source,
    ts.ScriptTarget.Latest,
    false,
    ts.ScriptKind.JS,
  );
  if (sourceFile.parseDiagnostics.length > 0) return null;

  let helperFunction = null;
  const cargoInvocations = [];
  const visit = (node, insideTargetLoop = false) => {
    const isTargetLoop = ts.isForOfStatement(node)
      && ts.isVariableDeclarationList(node.initializer)
      && node.initializer.declarations.length === 1
      && ts.isIdentifier(node.initializer.declarations[0].name)
      && node.initializer.declarations[0].name.text === 'target'
      && ts.isIdentifier(node.expression)
      && node.expression.text === 'targets';
    const currentInsideTargetLoop = insideTargetLoop || isTargetLoop;
    if (ts.isFunctionDeclaration(node)
      && node.name?.text === 'helperCargoArgsForTarget') {
      helperFunction = node;
    }
    if (ts.isCallExpression(node)
      && ts.isIdentifier(node.expression)
      && node.expression.text === 'run'
      && node.arguments.length >= 2
      && ts.isStringLiteral(node.arguments[0])
      && node.arguments[0].text === 'cargo') {
      cargoInvocations.push({ call: node, insideTargetLoop: currentInsideTargetLoop });
    }
    ts.forEachChild(node, (child) => visit(child, currentInsideTargetLoop));
  };
  visit(sourceFile);

  const cargoInvocation = cargoInvocations[0];
  const directHelperCall = cargoInvocation?.call.arguments[1];
  if (!helperFunction
    || cargoInvocations.length !== 1
    || !cargoInvocation.insideTargetLoop
    || !directHelperCall
    || !ts.isCallExpression(directHelperCall)
    || !ts.isIdentifier(directHelperCall.expression)
    || directHelperCall.expression.text !== 'helperCargoArgsForTarget'
    || directHelperCall.arguments.length !== 1
    || !ts.isIdentifier(directHelperCall.arguments[0])
    || directHelperCall.arguments[0].text !== 'target') {
    return null;
  }

  const helperSource = source.slice(helperFunction.getStart(sourceFile), helperFunction.end);
  const evaluate = (target) => {
    try {
      const result = vm.runInNewContext(
        `(${helperSource})(${JSON.stringify(target)})`,
        Object.create(null),
        { timeout: 50 },
      );
      return Array.isArray(result) && result.every((item) => typeof item === 'string')
        ? Array.from(result)
        : null;
    } catch {
      return null;
    }
  };

  return {
    arm64: evaluate('aarch64-apple-darwin'),
    intel: evaluate('x86_64-apple-darwin'),
  };
}

export function validateMacosRustSurface({
  workspaceCargoToml,
  appCargoToml,
  llamaCargoToml,
  packageJson,
  sourceFiles,
}) {
  const findings = [];
  const add = (code, filePath, message) => {
    findings.push(finding(
      code,
      filePath,
      message,
      'Restore the reviewed macOS-only target routing and rerun the release policy.',
    ));
  };

  const workspaceManifest = parseToml(workspaceCargoToml);
  if (!['2', '3'].includes(String(workspaceManifest.workspace?.resolver ?? ''))) {
    add('rust.workspace-resolver', 'Cargo.toml', 'Cargo resolver must isolate inactive target features.');
  }

  const appManifest = parseToml(appCargoToml);
  const targetEntries = Object.entries(appManifest.target ?? {});
  const allowedMacosTargets = new Set([
    'cfg(target_os="macos")',
    'cfg(all(target_os="macos",target_arch="aarch64"))',
  ]);
  for (const [target, table] of targetEntries) {
    const normalized = normalizedCargoTarget(target);
    const hasTargetDependencies = ['dependencies', 'build-dependencies', 'dev-dependencies']
      .some((key) => table?.[key]);
    if (hasTargetDependencies && !allowedMacosTargets.has(normalized)) {
      add('rust.non-macos-target', 'frontend/src-tauri/Cargo.toml', `Unreviewed target dependencies remain in ${target}.`);
    }
  }

  const forbiddenAppFeatures = new Set([
    'cuda', 'vulkan', 'hipblas', 'openblas', 'openmp', 'metal', 'coreml', 'platform-default',
  ]);
  for (const feature of Object.keys(appManifest.features ?? {})) {
    if (forbiddenAppFeatures.has(feature)) {
      add('rust.manual-acceleration-feature', 'frontend/src-tauri/Cargo.toml', `Manual app acceleration feature remains: ${feature}.`);
    }
  }

  const commonTarget = targetEntries.find(([target]) => (
    normalizedCargoTarget(target) === 'cfg(target_os="macos")'
  ));
  const armTarget = targetEntries.find(([target]) => {
    const normalized = normalizedCargoTarget(target);
    return normalized.includes('target_os="macos"') && normalized.includes('target_arch="aarch64"');
  });
  const intelTargets = targetEntries.filter(([target]) => (
    normalizedCargoTarget(target).includes('target_arch="x86_64"')
  ));
  const commonWhisper = commonTarget?.[1]?.dependencies?.['whisper-rs'];
  const armWhisper = armTarget?.[1]?.dependencies?.['whisper-rs'];
  const commonFeatures = cargoDependencyFeatures(commonWhisper);
  const armFeatures = cargoDependencyFeatures(armWhisper);

  const acceleratedIntelDependency = [commonWhisper, ...intelTargets.map(([, table]) => (
    table?.dependencies?.['whisper-rs']
  ))].some((dependency) => {
    const features = cargoDependencyFeatures(dependency);
    return features.has('metal') || features.has('coreml');
  });
  if (acceleratedIntelDependency) {
    add('rust.intel-whisper-acceleration', 'frontend/src-tauri/Cargo.toml', 'Intel/common Whisper enables Apple-Silicon acceleration.');
  }

  for (const dependency of [commonWhisper, armWhisper]) {
    if (!dependency || typeof dependency !== 'object' || dependency.version !== '0.13.2') {
      add('rust.whisper-source', 'frontend/src-tauri/Cargo.toml', 'A matching Whisper target entry is missing the pinned 0.13.2 source version.');
    }
  }
  if (!commonFeatures.has('raw-api') || !armFeatures.has('metal') || !armFeatures.has('coreml')) {
    add('rust.arm-whisper-acceleration', 'frontend/src-tauri/Cargo.toml', 'Whisper target features do not preserve raw-api plus arm64 Metal/CoreML.');
  }

  for (const dependency of ['tar', 'xz2', 'which']) {
    if (Object.hasOwn(appManifest['build-dependencies'] ?? {}, dependency)) {
      add('rust.non-macos-build-dependency', 'frontend/src-tauri/Cargo.toml', `Non-macOS build dependency remains: ${dependency}.`);
    }
  }

  const helperManifest = parseToml(llamaCargoToml);
  const helperFeatures = helperManifest.features ?? {};
  const helperFeatureNames = Object.keys(helperFeatures).sort();
  if (helperFeatureNames.join(',') !== 'default,metal'
    || !Array.isArray(helperFeatures.default)
    || helperFeatures.default.length !== 0
    || !Array.isArray(helperFeatures.metal)
    || helperFeatures.metal.length !== 1
    || helperFeatures.metal[0] !== 'llama-cpp-2/metal') {
    add('rust.helper-feature', 'llama-helper/Cargo.toml', 'llama-helper features must be exactly empty default plus Metal.');
  }

  const obsoleteWrappers = new Set([
    'frontend/build-gpu.sh',
    'frontend/dev-gpu.sh',
    'frontend/scripts/auto-detect-gpu.js',
    'frontend/scripts/tauri-auto.js',
  ]);
  for (const filePath of Object.keys(sourceFiles)) {
    if (obsoleteWrappers.has(filePath)) {
      add('rust.manual-gpu-script', filePath, 'Obsolete manual GPU wrapper remains tracked.');
    }
  }
  for (const [name, command] of Object.entries(packageJson.scripts ?? {})) {
    if (/--features\b/.test(command) || /^tauri:(?:dev|build):(?:cpu|cuda|vulkan|metal|coreml|openblas|hipblas)$/.test(name)) {
      add('rust.manual-gpu-script', 'frontend/package.json', `Package script manually selects acceleration: ${name}.`);
    }
  }

  const universalPath = 'frontend/scripts/build-universal-macos.js';
  const universalSource = sourceFiles[universalPath] ?? '';
  const helperRouting = universalHelperCargoRouting(universalSource);
  const expectedHelperArgs = (target, metal) => [
    'build', '--offline', '--release', '--package', 'llama-helper',
    '--target', target, '--no-default-features',
    ...(metal ? ['--features', 'metal'] : []),
  ];
  if (!helperRouting
    || JSON.stringify(helperRouting.intel) !== JSON.stringify(
      expectedHelperArgs('x86_64-apple-darwin', false),
    )
    || JSON.stringify(helperRouting.arm64) !== JSON.stringify(
      expectedHelperArgs('aarch64-apple-darwin', true),
    )) {
    add('rust.universal-helper-routing', universalPath, 'Universal helper build does not isolate arm64 Metal from Intel CPU.');
  }

  const guardedSourcePaths = [
    'frontend/src-tauri/build.rs',
    'frontend/src-tauri/build/ffmpeg.rs',
    'frontend/src-tauri/src/audio/ffmpeg.rs',
    'frontend/src-tauri/src/summary/summary_engine/sidecar.rs',
    'frontend/src-tauri/src/console_utils/console_utils.rs',
    'frontend/src-tauri/src/api/api.rs',
    'frontend/src-tauri/src/database/commands.rs',
    'frontend/src-tauri/src/whisper_engine/commands.rs',
    'frontend/src-tauri/src/parakeet_engine/commands.rs',
    'frontend/src-tauri/src/lib.rs',
    'frontend/src-tauri/src/main.rs',
  ];
  const forbiddenSource = /(?:pc-windows|unknown-linux|\.exe\b|\.tar\.xz\b|\btar::|\bxz2::|extract_tar|nvidia-smi|rocm-smi|vulkaninfo|\bnvcc\b|cfg\s*\(\s*(?:windows|target_os\s*=\s*"(?:windows|linux)")|windows_subsystem|Command::new\s*\(\s*"(?:cmd|explorer|powershell)"\s*\))/i;
  for (const filePath of guardedSourcePaths) {
    const source = rustWithoutComments(sourceFiles[filePath] ?? '');
    if (forbiddenSource.test(source)) {
      add('rust.non-macos-build-source', filePath, 'A guarded Rust/build source still contains non-macOS routing.');
    }
  }

  return findings;
}

export function validateMacosAudioSurface({ existingPaths, sourceFiles }) {
  const findings = [];
  const add = (code, filePath, message) => findings.push(finding(
    code,
    filePath,
    message,
    'Keep recording lifecycle behind the managed RecordingSession and the single bounded macOS capture adapter.',
  ));
  const requiredPaths = [
    'frontend/src-tauri/src/audio/recording_session.rs',
    'frontend/src-tauri/src/audio/recording_session_state.rs',
    'frontend/src-tauri/src/audio/capture/backend.rs',
    'frontend/src-tauri/src/audio/capture/macos.rs',
  ];
  for (const filePath of requiredPaths) {
    if (!existingPaths.has(filePath)) add('audio.missing-module', filePath, 'Required recording module is absent.');
  }

  const productionAdapterCount = Object.entries(sourceFiles)
    .filter(([filePath]) => filePath.startsWith('frontend/src-tauri/src/audio/capture/'))
    .reduce((count, [, source]) => count + (source.match(/\bstruct\s+MacosCaptureBackend\b/g) ?? []).length, 0);
  if (productionAdapterCount !== 1) {
    add('audio.capture-adapter-count', 'frontend/src-tauri/src/audio/capture', `Expected one MacosCaptureBackend, found ${productionAdapterCount}.`);
  }

  const sessionPath = 'frontend/src-tauri/src/audio/recording_session.rs';
  const session = sourceFiles[sessionPath] ?? '';
  const requiredMethods = ['start', 'pause', 'resume', 'stop', 'recover', 'get', 'get_active'];
  if (!/\bpub\s+struct\s+RecordingSession\b/.test(session)
    || requiredMethods.some((name) => !new RegExp(`\\bpub\\s+async\\s+fn\\s+${name}\\b`).test(session))) {
    add('audio.recording-session-interface', sessionPath, 'RecordingSession does not expose the complete lifecycle interface.');
  }

  const lib = sourceFiles['frontend/src-tauri/src/lib.rs'] ?? '';
  if (!/\.manage\s*\([^)]*RecordingSession::new\s*\(/s.test(lib)) {
    add('audio.unmanaged-session', 'frontend/src-tauri/src/lib.rs', 'Tauri does not manage the RecordingSession owner.');
  }

  for (const [filePath, source] of Object.entries(sourceFiles)) {
    if (/\b(?:IS_RECORDING|RECORDING_MANAGER|RECORDING_FLAG)\b/.test(rustWithoutComments(source))) {
      add('audio.lifecycle-global', filePath, 'A competing recording lifecycle global remains.');
    }
    if (filePath.startsWith('frontend/src-tauri/src/audio/capture/')
      && /unsafe\s+impl\s+(?:Send|Sync)\b/.test(rustWithoutComments(source))) {
      add('audio.unsafe-send-sync', filePath, 'Capture code contains an unsafe Send or Sync shim.');
    }
    if (/(?:\bunbounded_channel|std::sync::mpsc::channel)\s*(?:::|\()/m.test(source)
      && /(?:TrackChunk|CaptureControl|TrackSink)/.test(source)) {
      add('audio.unbounded-channel', filePath, 'Capture or Track transport uses an unbounded channel.');
    }
    if (filePath.startsWith('frontend/src/')
      && !filePath.endsWith('/services/recordingService.ts')
      && !filePath.includes('/testing/')
      && /invoke(?:<[^>]+>)?\s*\(\s*['"](?:start|stop|pause|resume)_recording/.test(source)) {
      add('audio.direct-ui-invoke', filePath, 'UI code invokes a recording action outside recordingService.');
    }
  }

  const backend = sourceFiles['frontend/src-tauri/src/audio/capture/backend.rs'] ?? '';
  if (!/CAPTURE_CONTROL_MAX_MESSAGES\s*:\s*usize\s*=\s*8/.test(backend)
    || !/sync_channel\s*\(\s*CAPTURE_CONTROL_MAX_MESSAGES\s*\)/.test(backend)
    || !/sync_channel\s*\(\s*1\s*\)/.test(backend)) {
    add('audio.unbounded-control', 'frontend/src-tauri/src/audio/capture/backend.rs', 'Control and reply channels are not fixed and bounded.');
  }
  const backendSource = rustStructuralSource(backend);
  const tupleBinding = '(?:(?:ref\\s+mut|mut|ref)\\s+)?';
  const runtimeEventBindings = backendSource.match(
    new RegExp(`\\blet\\s+(?:mut\\s+)?\\(\\s*${tupleBinding}runtime_event_tx\\s*,\\s*${tupleBinding}runtime_event_rx\\s*\\)\\s*(?::\\s*[^=;]+)?=\\s*[^;]+;`, 'g'),
  ) ?? [];
  const boundedRuntimeEvents = runtimeEventBindings.length > 0
    && runtimeEventBindings.every((binding) => /=\s*(?:(?:std\s*::\s*sync\s*::\s*)?mpsc\s*::\s*)?sync_channel\s*(?:::\s*<[^;(){}]+>)?\s*\(\s*CAPTURE_CONTROL_MAX_MESSAGES\s*\)\s*;/.test(binding));
  if (!boundedRuntimeEvents) {
    add('audio.unbounded-runtime-event', 'frontend/src-tauri/src/audio/capture/backend.rs', 'Capture runtime events do not use the bounded capture-control channel capacity.');
  }

  const permissionHookPath = 'frontend/src/hooks/usePermissionCheck.ts';
  const permissionHook = sourceFiles[permissionHookPath] ?? '';
  if (javaScriptInvokedCommands(permissionHookPath, permissionHook).has('get_audio_devices')) {
    add('audio.permission-device-inference', permissionHookPath, 'The shared permission hook infers authorization from audio-device enumeration.');
  }

  const macosPath = 'frontend/src-tauri/src/audio/capture/macos.rs';
  const macos = rustStructuralSource(sourceFiles[macosPath] ?? '');
  const visibility = '(?:pub(?:\\s*\\([^)]*\\))?\\s+)?';
  const threeAttempts = new RegExp(`\\b${visibility}const\\s+MICROPHONE_RECOVERY_MAX_ATTEMPTS\\s*:\\s*u8\\s*=\\s*3(?:_?u8)?\\s*;`).test(macos);
  const threeSecondWindow = new RegExp(`\\b${visibility}const\\s+MICROPHONE_RECOVERY_WINDOW\\s*:\\s*(?:std::time::)?Duration\\s*=\\s*(?:std::time::)?Duration::from_secs\\s*\\(\\s*3(?:_?u64)?\\s*\\)\\s*;`).test(macos);
  if (!threeAttempts || !threeSecondWindow) {
    add('audio.missing-microphone-recovery-bound', macosPath, 'Microphone recovery is not capped at three attempts within three seconds.');
  }
  const macosProduction = macos.split(/#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]/, 1)[0];
  const permissionDeniedArms = rustMatchArmBodies(
    macosProduction,
    'CoreAudioSetupError::PermissionDenied',
  );
  const wholeCaptureHelpers = new Set(
    [...rustNamedFunctionBodies(macosProduction)]
      .filter(([, bodies]) => bodies.some((body) => /\bCaptureCause::PermissionDenied\b/.test(body)))
      .map(([name]) => name),
  );
  const armBlocksCapture = (body) => /\bCaptureCause::PermissionDenied\b/.test(body)
    || [...wholeCaptureHelpers].some((name) => new RegExp(`\\b${name}\\s*(?:::\\s*<[^;(){}]+>)?\\s*\\(`).test(body));
  if (permissionDeniedArms.some(armBlocksCapture)) {
    add('audio.system-failure-blocks-recording', macosPath, 'A system-audio permission failure terminates whole-session capture.');
  }
  const stoppedBody = backend.match(/struct\s+CaptureStopped\s*\{([\s\S]*?)\}/)?.[1] ?? '';
  if (!/closed_sources/.test(stoppedBody)
    || /CapturedTrack|path|bytes|samples|checksum|finaliz/i.test(stoppedBody)) {
    add('audio.capture-stop-evidence', 'frontend/src-tauri/src/audio/capture/backend.rs', 'CaptureStopped leaks or omits post-finalization evidence.');
  }

  return findings;
}

export function validateUpdaterDisabled({
  packageJson,
  cargoToml,
  cargoLock,
  sourceFiles,
  tauriConfig,
  universalConfig,
  capabilityFiles,
  trackedFiles,
  workflowFiles,
}) {
  const findings = [];
  const add = (code, filePath, surface) => {
    findings.push(finding(
      code,
      filePath,
      `Legacy updater surface remains enabled: ${surface}.`,
      'Remove this exact legacy updater surface; do not replace it with a placeholder or disabled adapter.',
    ));
  };

  if (manifestHasDependency(packageJson, '@tauri-apps/plugin-updater')) {
    add('updater.frontend-dependency', 'frontend/package.json', '@tauri-apps/plugin-updater dependency');
  }
  if (manifestHasDependency(packageJson, '@tauri-apps/plugin-process')) {
    add('process.frontend-dependency', 'frontend/package.json', '@tauri-apps/plugin-process dependency');
  }

  const cargoDependencies = productionCargoDependencies(cargoToml);
  if (cargoDependencies.has('tauri-plugin-updater')) {
    add('updater.cargo-dependency', 'frontend/src-tauri/Cargo.toml', 'tauri-plugin-updater production dependency');
  }
  if (cargoDependencies.has('tauri-plugin-process')) {
    add('process.cargo-dependency', 'frontend/src-tauri/Cargo.toml', 'tauri-plugin-process production dependency');
  }

  const lockPackages = cargoLockPackages(cargoLock);
  if (lockPackages.has('tauri-plugin-updater')) {
    add('updater.cargo-lock', 'Cargo.lock', 'tauri-plugin-updater lock package');
  }
  if (lockPackages.has('tauri-plugin-process')) {
    add('process.cargo-lock', 'Cargo.lock', 'tauri-plugin-process lock package');
  }

  for (const [filePath, content] of Object.entries(sourceFiles)) {
    if (productionJavaScriptPath(filePath)) {
      const facts = javaScriptPolicyFacts(filePath, content);
      for (const specifier of facts.imports) {
        if (specifier === '@tauri-apps/plugin-updater') {
          add('updater.frontend-import', filePath, '@tauri-apps/plugin-updater import');
        } else if (specifier === '@tauri-apps/plugin-process') {
          add('process.frontend-import', filePath, '@tauri-apps/plugin-process import');
        } else if (UPDATER_LOCAL_MODULES.has(resolvedLocalModule(filePath, specifier))) {
          add('updater.local-import', filePath, `updater-owned module import ${specifier}`);
        }
      }
      const runtimeCommands = facts.runtimeStrings.filter((value) => (
        value.startsWith('plugin:updater|') || value.startsWith('plugin:process|')
      ));
      if (runtimeCommands.some((command) => command.startsWith('plugin:updater|'))) {
        add('updater.direct-command', filePath, 'direct plugin:updater command');
      }
      if (runtimeCommands.some((command) => command.startsWith('plugin:process|'))) {
        add('process.direct-command', filePath, 'direct plugin:process command');
      }
    }
  }

  const libPath = 'frontend/src-tauri/src/lib.rs';
  const libSource = rustWithoutComments(sourceFiles[libPath] ?? '');
  if (/\.plugin\s*\(\s*tauri_plugin_updater::/.test(libSource)) {
    add('updater.rust-registration', libPath, 'Rust updater plugin registration');
  }
  if (/\.plugin\s*\(\s*tauri_plugin_process::/.test(libSource)) {
    add('process.rust-registration', libPath, 'Rust process plugin registration');
  }

  const tauriConfigs = [
    ['frontend/src-tauri/tauri.conf.json', tauriConfig, 'base'],
    ['frontend/src-tauri/tauri.universal.conf.json', universalConfig, 'Universal'],
  ];
  for (const [filePath, config, label] of tauriConfigs) {
    for (const permission of embeddedPermissions(config)) {
      const identifier = permissionIdentifier(permission);
      if (identifier?.startsWith('updater:')) {
        add('updater.embedded-permission', filePath, 'embedded updater permission');
      } else if (identifier?.startsWith('process:')) {
        add('process.embedded-permission', filePath, 'embedded process permission');
      }
    }
    const updaterConfig = config.plugins?.updater;
    if (updaterConfig && Object.hasOwn(updaterConfig, 'endpoints')) {
      add('updater.config-endpoints', filePath, 'updater endpoints');
    }
    if (updaterConfig && Object.hasOwn(updaterConfig, 'pubkey')) {
      add('updater.config-pubkey', filePath, 'updater public key');
    }
    if (Object.hasOwn(config.bundle ?? {}, 'createUpdaterArtifacts')) {
      add('updater.config-artifacts', filePath, `${label} updater artifact flag`);
    }
  }
  for (const [filePath, content] of Object.entries(capabilityFiles)) {
    for (const permission of capabilityPermissions(content)) {
      const identifier = permissionIdentifier(permission);
      if (identifier?.startsWith('updater:')) {
        add('updater.capability-permission', filePath, 'standalone updater permission');
      } else if (identifier?.startsWith('process:')) {
        add('process.capability-permission', filePath, 'standalone process permission');
      }
    }
  }

  for (const filePath of UPDATER_OWNED_FILES) {
    if (trackedFiles.has(filePath)) add('updater.owned-file', filePath, 'tracked updater-owned file');
  }

  const layoutPath = 'frontend/src/app/layout.tsx';
  if (/<UpdateCheckProvider\b/.test(sourceFiles[layoutPath] ?? '')) {
    add('updater.layout-provider', layoutPath, 'root UpdateCheckProvider mount');
  }
  const aboutPath = 'frontend/src/components/About.tsx';
  const aboutSource = sourceFiles[aboutPath] ?? '';
  if (/updateService\.checkForUpdates\s*\(\s*true\s*\)/.test(aboutSource)) {
    add('updater.about-call', aboutPath, 'manual updateService check');
  }
  if (/Check for Updates/.test(aboutSource)) {
    add('updater.about-button', aboutPath, 'manual update button text');
  }
  if (/<UpdateDialog\b/.test(aboutSource)) {
    add('updater.about-dialog', aboutPath, 'UpdateDialog render');
  }

  const trayPath = 'frontend/src-tauri/src/tray.rs';
  const traySource = rustWithoutComments(sourceFiles[trayPath] ?? '');
  if (/["']check_updates["']\s*=>\s*check_updates_handler\s*\(\s*app\s*\)/.test(traySource)) {
    add('updater.tray-match', trayPath, 'tray updater match arm');
  }
  if (/\bfn\s+check_updates_handler(?:\s*<[^>]+>)?\s*\(/.test(traySource)) {
    add('updater.tray-handler', trayPath, 'tray updater handler');
  }
  if (/["']check-updates-from-tray["']/.test(traySource)) {
    add('updater.tray-event', trayPath, 'tray updater dispatch event');
  }
  if (/MenuItemBuilder::with_id\s*\(\s*["']check_updates["']\s*,\s*["']Check for Updates["']/.test(traySource)) {
    add('updater.tray-menu', trayPath, 'tray updater menu item');
  }

  if (trackedFiles.has('scripts/generate-update-manifest-github.js')) {
    add('updater.manifest-script', 'scripts/generate-update-manifest-github.js', 'updater manifest script');
  }
  if (trackedFiles.has('scripts/test-update-locally.js')) {
    add('updater.local-test-script', 'scripts/test-update-locally.js', 'local updater test script');
  }

  for (const [filePath, rawContent] of Object.entries(workflowFiles)) {
    const { keys, searchableText } = workflowPolicyFacts(filePath, rawContent);
    if (keys.has('TAURI_SIGNING_PRIVATE_KEY')) {
      add('updater.signing-private-key', filePath, 'TAURI_SIGNING_PRIVATE_KEY field');
    }
    if (keys.has('TAURI_SIGNING_PRIVATE_KEY_PASSWORD')) {
      add('updater.signing-password', filePath, 'TAURI_SIGNING_PRIVATE_KEY_PASSWORD field');
    }
    if (/\blatest\.json\b/.test(searchableText)) {
      add('updater.latest-manifest', filePath, 'latest.json manifest handling');
    }
    if (/\*\.app\.tar\.gz(?!\.sig)/.test(searchableText)) {
      add('updater.macos-archive', filePath, '*.app.tar.gz artifact glob');
    }
    if (/\*\.app\.tar\.gz\.sig/.test(searchableText)) {
      add('updater.macos-signature', filePath, '*.app.tar.gz.sig artifact glob');
    }
    if (/\*\.msi\.sig/.test(searchableText)) {
      add('updater.msi-signature', filePath, '*.msi.sig artifact glob');
    }
    if (/\*\.exe\.sig/.test(searchableText)) {
      add('updater.exe-signature', filePath, '*.exe.sig artifact glob');
    }
  }

  return findings;
}

export function validateProductionConfig({
  existingPaths,
  nextConfig,
  packageJson,
  tauriConfig,
  universalConfig,
}) {
  const findings = [];
  const add = (code, configPath, message, remediation) => {
    findings.push(finding(code, configPath, message, remediation));
  };

  if (nextConfig.output !== 'export') {
    add(
      'next.static-export',
      'next.config.js',
      "Next.js production output must be the static 'export' used by the Tauri bundle.",
      "Set output: 'export' and verify that pnpm build creates frontend/out.",
    );
  }

  if (packageJson.version !== tauriConfig.version) {
    add(
      'config.version-mismatch',
      'package.json',
      `package.json version ${packageJson.version ?? '<missing>'} differs from Tauri version ${tauriConfig.version ?? '<missing>'}.`,
      'Set one reviewed release version in both package.json and src-tauri/tauri.conf.json.',
    );
  }

  if (tauriConfig.build?.frontendDist !== '../out') {
    add(
      'tauri.frontend-dist',
      'src-tauri/tauri.conf.json',
      "Tauri frontendDist must point to the packaged static '../out' directory, not a server.",
      "Set build.frontendDist to '../out'; devUrl is allowed only for tauri dev.",
    );
  }

  if (tauriConfig.build?.beforeBuildCommand !== 'pnpm build') {
    add(
      'tauri.before-build',
      'src-tauri/tauri.conf.json',
      'Tauri must run the reviewed production frontend build before bundling.',
      "Set build.beforeBuildCommand to 'pnpm build'.",
    );
  }

  if (!tauriConfig.bundle?.active) {
    add(
      'bundle.inactive',
      'src-tauri/tauri.conf.json',
      'Tauri bundling is disabled in the production configuration.',
      'Set bundle.active to true for production candidates.',
    );
  }

  for (const requiredBinary of ['binaries/llama-helper', 'binaries/ffmpeg']) {
    if (!tauriConfig.bundle?.externalBin?.includes(requiredBinary)) {
      add(
        'bundle.missing-external-bin',
        'src-tauri/tauri.conf.json',
        `Required sidecar declaration is missing: ${requiredBinary}.`,
        'Restore the externalBin declaration and verify the built sidecar in the application bundle.',
      );
    }
  }

  for (const icon of tauriConfig.bundle?.icon ?? []) {
    const iconPath = `src-tauri/${icon}`;
    if (!existingPaths.has(iconPath)) {
      add(
        'bundle.missing-icon',
        iconPath,
        `Declared application icon is missing: ${iconPath}.`,
        'Restore the declared icon or update the Tauri icon list to an existing reviewed asset.',
      );
    }
  }

  for (const [resource] of resourceEntries(tauriConfig.bundle?.resources)) {
    const resourcePattern = `src-tauri/${resource}`;
    if (![...existingPaths].some((candidate) => resourceSourceMatches(resourcePattern, candidate))) {
      add(
        'bundle.missing-resource',
        resourcePattern,
        `Declared bundle resource has no matching files: ${resourcePattern}.`,
        'Restore at least one required resource and verify it is present in the built application.',
      );
    }
  }

  if (tauriConfig.bundle?.resources?.[DIARIZATION_RESOURCE_SOURCE]
    !== DIARIZATION_RESOURCE_TARGET) {
    add(
      'bundle.missing-diarization-resources',
      'src-tauri/tauri.conf.json',
      'Pinned offline diarization models and license notices are not declared as resources.',
      `Map '${DIARIZATION_RESOURCE_SOURCE}' to '${DIARIZATION_RESOURCE_TARGET}' and provision the verified local artifact store.`,
    );
  }

  if (universalConfig.bundle?.resources?.[DIARIZATION_RESOURCE_SOURCE]
    !== DIARIZATION_RESOURCE_TARGET) {
    add(
      'universal.missing-diarization-resources',
      'src-tauri/tauri.universal.conf.json',
      'The Universal override does not preserve the pinned diarization resources.',
      `Map '${DIARIZATION_RESOURCE_SOURCE}' to '${DIARIZATION_RESOURCE_TARGET}' in the Universal bundle override.`,
    );
  }

  if (tauriConfig.bundle?.macOS?.hardenedRuntime !== true) {
    add(
      'bundle.hardened-runtime',
      'src-tauri/tauri.conf.json',
      'macOS hardened runtime is not enabled.',
      'Set bundle.macOS.hardenedRuntime to true and re-run signing verification.',
    );
  }

  if (tauriConfig.bundle?.macOS?.minimumSystemVersion !== '14.2') {
    add(
      'bundle.minimum-system-version',
      'src-tauri/tauri.conf.json',
      `macOS deployment target must be 14.2, got ${tauriConfig.bundle?.macOS?.minimumSystemVersion ?? '<missing>'}.`,
      'Set bundle.macOS.minimumSystemVersion to 14.2 so the packaged app matches the supported macOS baseline.',
    );
  }

  const universalMinimumSystemVersion = universalConfig.bundle?.macOS?.minimumSystemVersion;
  if (universalMinimumSystemVersion !== undefined && universalMinimumSystemVersion !== '14.2') {
    add(
      'universal.minimum-system-version',
      'src-tauri/tauri.universal.conf.json',
      `Universal macOS deployment target must be 14.2 when overridden, got ${universalMinimumSystemVersion}.`,
      'Remove the conflicting override or set bundle.macOS.minimumSystemVersion to 14.2.',
    );
  }

  const signingIdentity = tauriConfig.bundle?.macOS?.signingIdentity;
  if (typeof signingIdentity === 'string' && signingIdentity !== '-' && /[/\\]/.test(signingIdentity)) {
    add(
      'bundle.signing-identity',
      'src-tauri/tauri.conf.json',
      'The macOS signing identity contains a developer-machine path.',
      'Use an identity name or CI secret-backed signing setup; never commit certificate paths.',
    );
  }

  if (tauriConfig.productName !== 'meetily' || tauriConfig.identifier !== 'com.meetily.ai') {
    add(
      'bundle.identity',
      'src-tauri/tauri.conf.json',
      'Product name or bundle identifier differs from the reviewed Meetily identity.',
      "Use productName 'meetily' and identifier 'com.meetily.ai', or review and migrate identity explicitly.",
    );
  }

  return findings;
}

function permitsCredentialReference(line) {
  const isReference = /process\.env|std::env::var|env!\s*\(|secrets\.|\$\{\{\s*secrets\.|pub(?:lic)?[_-]?key/i.test(line);
  // PostHog documents phc_ project tokens as public ingestion identifiers.
  // Personal and secure PostHog keys use other prefixes and remain blocked.
  const isPublicPostHogProjectToken = /\b(?:api[_-]?key|project[_-]?token)\b\s*[:=]\s*["']phc_[A-Za-z0-9_-]{20,}["']/i.test(line);
  return isReference || isPublicPostHogProjectToken;
}

function isProductionSource(filePath) {
  const normalized = filePath.replaceAll('\\', '/');
  return /(^|\/)src\//.test(normalized) && !/(^|\/)src\/testing\//.test(normalized);
}

function canAffectProductionBuild(filePath) {
  const normalized = filePath.replaceAll('\\', '/');
  return (
    normalized.startsWith('src/')
    || normalized.startsWith('frontend/src/')
    || normalized.startsWith('frontend/src-tauri/src/')
    || normalized.startsWith('frontend/src-tauri/build/')
    || normalized.startsWith('frontend/scripts/')
    || normalized.startsWith('.github/workflows/')
    || normalized === 'frontend/next.config.js'
    || normalized.startsWith('frontend/src-tauri/tauri.')
  );
}

export function scanTextFile(filePath, content) {
  const findings = [];
  const privateKeyMarker = new RegExp(`-----BEGIN (?:RSA |EC |OPENSSH )?${'PRIVATE KEY'}-----`);
  if (privateKeyMarker.test(content)) {
    findings.push(finding(
      'secret.private-key',
      filePath,
      'A private-key block appears in a tracked text file.',
      'Remove the key from Git history, rotate it, and load replacement credentials from a secret store.',
    ));
  }

  const credentialPattern = /\b(?:api[_-]?key|access[_-]?token|auth[_-]?token|client[_-]?secret|password|secret)\b\s*[:=]\s*["'][A-Za-z0-9_+./=-]{20,}["']/gi;
  for (const line of content.split(/\r?\n/)) {
    credentialPattern.lastIndex = 0;
    if (credentialPattern.test(line) && !permitsCredentialReference(line)) {
      findings.push(finding(
        'secret.credential-literal',
        filePath,
        'A high-confidence credential literal appears in tracked source.',
        'Remove and rotate the credential, then read it from an environment variable or platform secret store.',
      ));
      break;
    }
  }

  const developerPathContent = filePath === 'frontend/scripts/verify-release-candidate.mjs'
    ? REVIEWED_CANDIDATE_VERIFIER_PATHS.reduce(
      (remaining, reviewedPath) => remaining.replaceAll(reviewedPath, '<audited-third-party-path>'),
      content,
    )
    : content;
  if (
    canAffectProductionBuild(filePath)
    && (/(?:^|[^A-Za-z])\/Users\/[^/\s]+\//m.test(developerPathContent)
      || /[A-Za-z]:\\+Users\\+[^\\\s]+\\+/i.test(developerPathContent))
  ) {
    findings.push(finding(
      'path.developer-home',
      filePath,
      'A developer-home absolute path is embedded in tracked text.',
      'Resolve the path from Tauri platform directories, configuration, or the current workspace at runtime.',
    ));
  }

  if (
    isProductionSource(filePath)
    && /https?:\/\/(?:localhost|127\.0\.0\.1):3118\b/.test(content)
  ) {
    findings.push(finding(
      'production.dev-server',
      filePath,
      'Production source references the Next.js development server on port 3118.',
      'Use packaged frontend assets; keep the port only in Tauri devUrl or development tooling.',
    ));
  }

  return findings;
}

function trackedFiles() {
  return execFileSync('git', ['ls-files', '-z'], {
    cwd: repositoryRoot,
    encoding: 'utf8',
  }).split('\0').filter(Boolean);
}

export function validateLegacyRuntimeAbsent({
  trackedFiles = new Set(),
  sourceFiles = {},
  packageJson = {},
  tauriConfig = {},
  workflowFiles = {},
} = {}) {
  const findings = [];
  const tracked = trackedFiles instanceof Set ? trackedFiles : new Set(trackedFiles);
  const productionSourcePath = (filePath) => {
    const normalized = filePath.replaceAll('\\', '/');
    const underProductSource = normalized.startsWith('frontend/src/')
      || normalized.startsWith('frontend/src-tauri/src/');
    return underProductSource
      && !normalized.startsWith('frontend/src/testing/')
      && !normalized.startsWith('frontend/src-tauri/tests/')
      && !/(?:^|\/)tests?\//.test(normalized)
      && !/\.(?:test|spec)\.[cm]?[jt]sx?$/.test(normalized);
  };
  const add = (code, filePath, message, remediation) => {
    findings.push(finding(code, filePath, message, remediation));
  };

  if ([...tracked].some((filePath) => filePath === 'backend' || filePath.startsWith('backend/'))) {
    add(
      'legacy.backend-tree',
      'backend/',
      'The tracked product backend tree is still present.',
      'Remove the reviewed legacy backend paths after native behavior and data compatibility are verified.',
    );
  }

  const gitmodules = sourceFiles['.gitmodules'] ?? '';
  if (tracked.has('.gitmodules') && /(?:^|\n)\s*(?:path\s*=\s*)?backend(?:\/|\b)/m.test(gitmodules)) {
    add(
      'legacy.backend-gitlink',
      '.gitmodules',
      'A Git submodule still points into the legacy backend tree.',
      'Remove the backend gitlink and its module declaration together.',
    );
  }

  if (tracked.has('frontend/src-tauri/config/backend_config.json')) {
    add(
      'legacy.launch-surface',
      'frontend/src-tauri/config/backend_config.json',
      'The obsolete product backend configuration is still tracked.',
      'Delete the unused backend configuration after native command migration is complete.',
    );
  }

  const launchPattern = /(?:\bpython(?:3(?:\.\d+)?)?\b[^\r\n]*(?:\.py[cowz]?\b|-\s*m\s+[A-Za-z_]\w*(?:\.\w+)*|backend|fastapi|uvicorn)|\buvicorn\b|\bfastapi\b|\bdocker(?:-compose)?\b.*\bbackend\b)/i;
  for (const [scriptName, command] of Object.entries(packageJson.scripts ?? {})) {
    if (typeof command === 'string'
      && (launchPattern.test(command) || LEGACY_SCRIPT_TOKEN.test(command))) {
      add(
        'legacy.launch-surface',
        'frontend/package.json',
        `Package script ${scriptName} launches the legacy Python/FastAPI/Docker backend.`,
        'Remove product backend launch scripts; keep only explicit build and development tools.',
      );
      break;
    }
  }
  for (const [filePath, content] of Object.entries(workflowFiles)) {
    if (executableWorkflowCommands(filePath, content).some(({ kind, command }) => (
      launchPattern.test(command)
      || LEGACY_SCRIPT_TOKEN.test(command)
      || (kind === 'shell' && LEGACY_EXECUTABLE.test(executableBasename(command)))
    ))) {
      add(
        'legacy.launch-surface',
        filePath,
        'A workflow launches or packages the legacy Python/FastAPI/Docker backend.',
        'Remove product runtime launch and packaging steps while retaining unrelated build tools.',
      );
    }
  }

  const forbiddenPort = /(?:localhost|127\.0\.0\.1):(?:5167|8178)\b/;
  const httpOrchestration = /\b(?:APP_SERVER_URL|api_get_profile|api_save_profile|api_update_profile|test_backend_connection|debug_backend_connection)\b/;
  const backendJoin = /(?:\.join\(\s*["']backend(?:\/[^"']*)?["']\s*\)|\.push\(\s*["']backend(?:\/[^"']*)?["']\s*\)|\b(?:Path|PathBuf)::(?:from|new)\(\s*["']backend(?:\/[^"']*)?["']\s*\))/g;
  const exactMigrationLocator = /\blet\s+backend_db\s*=\s*path\s*\.join\(\s*["']backend["']\s*\)\s*\.join\(\s*["']meeting_minutes\.db["']\s*\)\s*;/g;
  const migrationLocatorMutation = /\bbackend_db\s*\.\s*(?:push|join|with_extension|set_extension)\s*\(/;
  const exactMigrationLocatorRange = (content) => {
    const source = rustWithoutComments(content);
    const locators = [...source.matchAll(exactMigrationLocator)];
    exactMigrationLocator.lastIndex = 0;
    if (locators.length !== 1) return null;
    const locator = locators[0];
    const functionPattern = /\b(?:pub\s+)?(?:async\s+)?fn\s+detect_legacy_database\s*\([^)]*\)[^{]*\{/g;
    for (const functionMatch of source.matchAll(functionPattern)) {
      const open = functionMatch.index + functionMatch[0].lastIndexOf('{');
      let depth = 0;
      let quote = null;
      let escaped = false;
      for (let index = open; index < source.length; index += 1) {
        const character = source[index];
        if (quote) {
          if (character === '\\' && !escaped) {
            escaped = true;
            continue;
          }
          if (character === quote && !escaped) quote = null;
          escaped = false;
          continue;
        }
        if (character === '"' || character === "'") quote = character;
        else if (character === '{') depth += 1;
        else if (character === '}' && --depth === 0) {
          if (locator.index > open && locator.index + locator[0].length <= index) {
            return {
              source,
              index: locator.index,
              length: locator[0].length,
              functionStart: open,
              functionEnd: index,
            };
          }
          break;
        }
      }
    }
    return null;
  };
  for (const [filePath, originalContent] of Object.entries(sourceFiles)) {
    if (!productionSourcePath(filePath)) continue;
    const content = String(originalContent);
    const launchesLegacyRuntime = filePath.endsWith('.rs')
      ? rustLaunchesLegacyRuntime(filePath, content)
      : /\.(?:[cm]?[jt]sx?)$/.test(filePath)
        ? javaScriptLaunchesLegacyRuntime(filePath, content)
        : false;
    if (launchesLegacyRuntime) {
      add(
        'legacy.launch-surface',
        filePath,
        'Production source launches the legacy Python/FastAPI/Docker backend.',
        'Remove product backend process launch code; keep Python only in explicit development tooling.',
      );
    }
    if (forbiddenPort.test(content)) {
      add(
        'legacy.local-port',
        filePath,
        'Production source references legacy local product ports 5167 or 8178.',
        'Use the registered native Tauri command and remove the obsolete loopback address.',
      );
    }
    if (httpOrchestration.test(content)) {
      add(
        'legacy.http-orchestration',
        filePath,
        'Production source still contains a caller-free local backend HTTP command or helper.',
        'Delete only the reviewed legacy HTTP commands and retain supported provider networking.',
      );
    }

    const locatorRange = filePath === 'frontend/src-tauri/src/database/commands.rs'
      ? exactMigrationLocatorRange(content)
      : null;
    const backendSource = filePath.endsWith('.rs') ? rustWithoutComments(content) : content;
    const withoutExactLocator = locatorRange
      ? `${locatorRange.source.slice(0, locatorRange.index)}${locatorRange.source.slice(locatorRange.index + locatorRange.length)}`
      : backendSource;
    const invalidLocatorUse = locatorRange
      ? [...locatorRange.source.matchAll(/\bbackend_db\b/g)].some((match) => {
        if (match.index >= locatorRange.index
          && match.index < locatorRange.index + locatorRange.length) return false;
        if (match.index <= locatorRange.functionStart || match.index >= locatorRange.functionEnd) {
          return true;
        }
        const suffix = locatorRange.source.slice(match.index + match[0].length);
        return !/^\s*\.\s*(?:exists|is_file|to_string_lossy)\s*\(/.test(suffix);
      })
      : false;
    if (backendJoin.test(withoutExactLocator)
      || (filePath === 'frontend/src-tauri/src/database/commands.rs'
        && (migrationLocatorMutation.test(content) || invalidLocatorUse))) {
      add(
        'legacy.backend-path',
        filePath,
        'Production source constructs a backend path outside the exact migration-only locator.',
        'Remove the runtime backend path or keep only database/commands.rs backend/meeting_minutes.db migration detection.',
      );
    }
    backendJoin.lastIndex = 0;
  }

  const serializedTauriConfig = JSON.stringify(tauriConfig);
  if (forbiddenPort.test(serializedTauriConfig)) {
    add(
      'legacy.local-port',
      'frontend/src-tauri/tauri.conf.json',
      'Tauri configuration still permits a legacy local product port.',
      'Remove only ports 5167 and 8178 while preserving reviewed provider endpoints.',
    );
  }
  const bundleEntries = [
    ...(tauriConfig.bundle?.externalBin ?? []),
    ...resourceEntries(tauriConfig.bundle?.resources).flatMap(([source, target]) => [source, target]),
  ].filter((entry) => typeof entry === 'string');
  if (bundleEntries.some((entry) => (
    /\.py[cowz]?(?:$|[?*])/i.test(entry)
    || /(?:^|\/)(?:backend|python(?:3(?:\.\d+)?)?|fastapi|docker)(?:\/|\b)/i.test(entry)
  ))) {
    add(
      'legacy.bundle-runtime',
      'frontend/src-tauri/tauri.conf.json',
      'Tauri bundle declarations include legacy backend runtime content.',
      'Bundle only the reviewed native helper, FFmpeg, models, and application resources.',
    );
  }

  return findings;
}

function existingFrontendPaths(files) {
  return new Set(
    files
      .filter((filePath) => filePath.startsWith('frontend/'))
      .map((filePath) => filePath.slice('frontend/'.length)),
  );
}

function shouldScan(filePath) {
  const extension = path.extname(filePath).toLowerCase();
  if (!TEXT_EXTENSIONS.has(extension)) return false;
  if (/pnpm-lock\.yaml$/.test(filePath)) return false;
  try {
    return statSync(path.join(repositoryRoot, filePath)).size <= 1_000_000;
  } catch {
    return false;
  }
}

export function validateRepository({ sourceFiles: sourceOverrides = {} } = {}) {
  const files = trackedFiles().filter((filePath) => existsSync(path.join(repositoryRoot, filePath)));
  const packageJson = JSON.parse(readFileSync(path.join(frontendRoot, 'package.json'), 'utf8'));
  const nextConfigSource = readFileSync(path.join(frontendRoot, 'next.config.js'), 'utf8');
  const nextConfig = {
    output: /\boutput\s*:\s*['"]export['"]/.test(nextConfigSource) ? 'export' : undefined,
  };
  const tauriConfig = JSON.parse(readFileSync(
    path.join(frontendRoot, 'src-tauri', 'tauri.conf.json'),
    'utf8',
  ));
  const universalConfig = JSON.parse(readFileSync(
    path.join(frontendRoot, 'src-tauri', 'tauri.universal.conf.json'),
    'utf8',
  ));

  const findings = validateProductionConfig({
    existingPaths: existingFrontendPaths(files),
    nextConfig,
    packageJson,
    tauriConfig,
    universalConfig,
  });

  const textByPath = (selectedFiles) => Object.fromEntries(selectedFiles.map((filePath) => [
    filePath,
    sourceOverrides[filePath] ?? readFileSync(path.join(repositoryRoot, filePath), 'utf8'),
  ]));
  const sourceFiles = files.filter((filePath) => (
    /\.(?:[cm]?[jt]sx?|rs)$/.test(filePath)
  ));
  const sourceTextByPath = textByPath(sourceFiles);
  const macosAudioSurfaceFiles = sourceFiles.filter((filePath) => (
    filePath.startsWith('frontend/src/') || filePath.startsWith('frontend/src-tauri/src/')
  ));
  const capabilityFiles = files.filter((filePath) => (
    filePath.startsWith('frontend/src-tauri/capabilities/') && filePath.endsWith('.json')
  ));
  const workflowFiles = files.filter((filePath) => (
    filePath.startsWith('.github/workflows/') || filePath === 'frontend/.env.example'
  ));
  findings.push(...validateUpdaterDisabled({
    packageJson,
    cargoToml: readFileSync(path.join(frontendRoot, 'src-tauri', 'Cargo.toml'), 'utf8'),
    cargoLock: readFileSync(path.join(repositoryRoot, 'Cargo.lock'), 'utf8'),
    sourceFiles: sourceTextByPath,
    tauriConfig,
    universalConfig,
    capabilityFiles: textByPath(capabilityFiles),
    trackedFiles: new Set(files),
    workflowFiles: textByPath(workflowFiles),
  }));
  const macosRustSurfacePaths = [
    'frontend/package.json',
    'frontend/build-gpu.sh',
    'frontend/dev-gpu.sh',
    'frontend/scripts/auto-detect-gpu.js',
    'frontend/scripts/tauri-auto.js',
    'frontend/scripts/build-universal-macos.js',
    'frontend/src-tauri/build.rs',
    'frontend/src-tauri/build/ffmpeg.rs',
    'frontend/src-tauri/src/audio/ffmpeg.rs',
    'frontend/src-tauri/src/summary/summary_engine/sidecar.rs',
    'frontend/src-tauri/src/console_utils/console_utils.rs',
    'frontend/src-tauri/src/api/api.rs',
    'frontend/src-tauri/src/database/commands.rs',
    'frontend/src-tauri/src/whisper_engine/commands.rs',
    'frontend/src-tauri/src/parakeet_engine/commands.rs',
    'frontend/src-tauri/src/lib.rs',
    'frontend/src-tauri/src/main.rs',
  ].filter((filePath) => files.includes(filePath));
  findings.push(...validateMacosRustSurface({
    workspaceCargoToml: readFileSync(path.join(repositoryRoot, 'Cargo.toml'), 'utf8'),
    appCargoToml: readFileSync(path.join(frontendRoot, 'src-tauri', 'Cargo.toml'), 'utf8'),
    llamaCargoToml: readFileSync(path.join(repositoryRoot, 'llama-helper', 'Cargo.toml'), 'utf8'),
    packageJson,
    sourceFiles: textByPath(macosRustSurfacePaths),
  }));
  findings.push(...validateMacosAudioSurface({
    existingPaths: new Set(files),
    sourceFiles: textByPath(macosAudioSurfaceFiles),
  }));
  const legacyRuntimeTextFiles = files.filter((filePath) => (
    filePath === '.gitmodules' || shouldScan(filePath)
  ));
  findings.push(...validateLegacyRuntimeAbsent({
    trackedFiles: new Set(files),
    sourceFiles: textByPath(legacyRuntimeTextFiles),
    packageJson,
    tauriConfig,
    workflowFiles: textByPath(workflowFiles),
  }));

  for (const filePath of files.filter(shouldScan)) {
    const content = readFileSync(path.join(repositoryRoot, filePath), 'utf8');
    findings.push(...scanTextFile(filePath, content));
  }

  return findings;
}

function main() {
  const findings = validateRepository();
  console.log('Meetily production policy and accidental-secret check');
  console.log('Scope: deterministic high-confidence checks; this is not a complete security audit.');

  if (findings.length === 0) {
    console.log('PASS: production configuration and tracked-source policy checks passed.');
    return;
  }

  for (const item of findings) {
    console.error(`FAIL [${item.code}] ${item.path}`);
    console.error(`  ${item.message}`);
    console.error(`  Fix: ${item.remediation}`);
  }
  console.error(`FAIL: ${findings.length} production policy finding(s).`);
  process.exitCode = 1;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
