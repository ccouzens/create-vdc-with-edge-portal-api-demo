/// <reference lib="dom" />

import { vcloud } from "@vcd/bindings";
import { DefaultApi } from "@ccouzens/ukcloud-portal-api";

const go = async (
  portalEmail: string,
  password: string,
  vcloudUrl: string,
  vcloudUsername: string,
  vcloudOrg: string,
  loginUrlCallback: (loginUrl: string) => void,
  vCloudFullUsernameCallback: (vcloudFullUsername: string) => void
) => {
  const vcloudFullUsername = `${vcloudUsername}@${vcloudOrg}`;
  vCloudFullUsernameCallback(vcloudFullUsername);
  const vcloudUrlUrl = new URL(vcloudUrl);
  vcloudUrlUrl.pathname = "/api/versions";

  const response = await fetch(vcloudUrlUrl.href, {
    headers: { Accept: "application/*+json" }
  });
  const versions: vcloud.api.rest.schema.versioning.SupportedVersionsType = await response.json();
  if (versions.versionInfo === undefined) {
    throw "Expected versionInfo to be defined";
  }
  const version32 = versions.versionInfo.find(v => v.version === "32.0");
  if (version32 === undefined) {
    throw "Expected version32 to be defined";
  }
  if (version32.loginUrl === undefined) {
    throw "Expected version32 to have a loginUrl";
  }
  loginUrlCallback(version32.loginUrl);

  const api = new DefaultApi();
  console.log(
    await api.apiAuthenticatePost({ email: portalEmail, password: password })
  );
};

const inputValue = (id: string): string => {
  const element = document.getElementsByTagName("input").namedItem(id);
  if (element === null) {
    throw `Expected document.getElementById(${id}) to not be null`;
  }
  return element.value;
};

const textCallback = (id: string): ((text: string) => void) => {
  const element = document.getElementById(id);
  if (element === null) {
    throw `Expected document.getElementById(${id}) to not be null`;
  }
  return (text: string) => {
    element.textContent = text;
  };
};

(async () => {
  await navigator.serviceWorker.register("/serviceWorker.js");

  const formEl = document
    .getElementsByTagName("form")
    .namedItem("create_vdc_with_edge_form");
  if (formEl === null) {
    throw "Expected on page form";
  }

  formEl.addEventListener("submit", event => {
    event.preventDefault();
    go(
      inputValue("portal_email"),
      inputValue("password"),
      inputValue("vcloud_url"),
      inputValue("vcloud_username"),
      inputValue("vcloud_org"),
      textCallback("vcloud_login_url"),
      textCallback("vcloud_full_username")
    ).catch(err => console.log(err));
  });
})().catch(err => console.log(err));
