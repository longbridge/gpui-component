#!/usr/bin/env bash
set -euo pipefail

# 用法：
# 1) 修改 TAG 变量后执行：script/release-tag.sh
# 2) 直接传参覆盖 TAG：script/release-tag.sh v0.1.0
# 3) 若需要覆盖同名 tag：FORCE_RETAG=true script/release-tag.sh v0.1.0
# 4) 脚本会自动同步 main/Cargo.toml 版本并提交后再推送分支和 tag

TAG="${1:-v0.1.0}"
REMOTE="${REMOTE:-origin}"
BRANCH="${BRANCH:-$(git rev-parse --abbrev-ref HEAD)}"
FORCE_RETAG="${FORCE_RETAG:-false}"
ALLOW_DIRTY="${ALLOW_DIRTY:-false}"
MAIN_MANIFEST="${MAIN_MANIFEST:-../main/Cargo.toml}"
RELEASE_VERSION="${TAG#v}"

update_main_version() {
  local manifest_path="$1"
  local new_version="$2"
  local current_version
  local temp_file

  if [[ ! -f "${manifest_path}" ]]; then
    echo "错误：未找到 main manifest：${manifest_path}"
    exit 1
  fi

  current_version="$(
    awk -F'"' '
      /^\[package\]$/ { in_package = 1; next }
      /^\[/ { in_package = 0 }
      in_package && /^version = "/ { print $2; exit }
    ' "${manifest_path}"
  )"

  if [[ -z "${current_version}" ]]; then
    echo "错误：无法从 ${manifest_path} 解析当前版本号。"
    exit 1
  fi

  if [[ "${current_version}" == "${new_version}" ]]; then
    echo "main 版本已是 ${new_version}，跳过提交。"
    return 1
  fi

  temp_file="$(mktemp)"
  if ! awk -v new_version="${new_version}" '
    BEGIN { in_package = 0; updated = 0 }
    /^\[package\]$/ { in_package = 1; print; next }
    /^\[/ { if (in_package) in_package = 0 }
    in_package && !updated && /^version = "/ {
      print "version = \"" new_version "\""
      updated = 1
      next
    }
    { print }
    END {
      if (!updated) {
        exit 1
      }
    }
  ' "${manifest_path}" > "${temp_file}"; then
    rm -f "${temp_file}"
    echo "错误：更新 ${manifest_path} 失败。"
    exit 1
  fi

  mv "${temp_file}" "${manifest_path}"
  echo "已更新 main 版本：${current_version} -> ${new_version}"
  return 0
}

echo "准备发布：tag=${TAG} branch=${BRANCH} remote=${REMOTE}"

if [[ ! "${TAG}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
  echo "错误：TAG 格式非法。示例：v0.1.0 或 v0.1.0-rc.1"
  exit 1
fi

if [[ "${ALLOW_DIRTY}" != "true" ]] && [[ -n "$(git status --porcelain)" ]]; then
  echo "错误：工作区不干净，请先提交或暂存变更。"
  echo "如确需跳过，可使用：ALLOW_DIRTY=true script/release-tag.sh ${TAG}"
  exit 1
fi

if git rev-parse -q --verify "refs/tags/${TAG}" >/dev/null; then
  if [[ "${FORCE_RETAG}" == "true" ]]; then
    echo "本地存在同名标签，正在删除：${TAG}"
    git tag -d "${TAG}"
  else
    echo "错误：本地已存在标签 ${TAG}。"
    echo "如需覆盖，请使用：FORCE_RETAG=true script/release-tag.sh ${TAG}"
    exit 1
  fi
fi

REMOTE_TAG_EXISTS="false"
if git ls-remote --tags "${REMOTE}" "refs/tags/${TAG}" | grep -q "${TAG}"; then
  REMOTE_TAG_EXISTS="true"
fi

if [[ "${REMOTE_TAG_EXISTS}" == "true" ]]; then
  if [[ "${FORCE_RETAG}" == "true" ]]; then
    echo "远端存在同名标签，正在删除：${TAG}"
    git push "${REMOTE}" ":refs/tags/${TAG}"
  else
    echo "错误：远端已存在标签 ${TAG}。"
    echo "如需覆盖，请使用：FORCE_RETAG=true script/release-tag.sh ${TAG}"
    exit 1
  fi
fi

if update_main_version "${MAIN_MANIFEST}" "${RELEASE_VERSION}"; then
  echo "提交 main 版本变更"
  git add "${MAIN_MANIFEST}"
  git commit -m "chore(main): bump version to ${RELEASE_VERSION}"
fi

echo "推送分支：${BRANCH}"
git push "${REMOTE}" "${BRANCH}"

echo "创建并推送标签：${TAG}"
git tag -a "${TAG}" -m "${TAG}"
git push "${REMOTE}" "${TAG}"

echo "完成：已触发 GitHub Actions Release 流程。"
echo "请在 GitHub Actions 查看 release.yml 运行状态。"
