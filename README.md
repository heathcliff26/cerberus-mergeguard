[![CI](https://github.com/heathcliff26/cerberus-mergeguard/actions/workflows/ci.yaml/badge.svg?event=push)](https://github.com/heathcliff26/cerberus-mergeguard/actions/workflows/ci.yaml)
[![Coverage Status](https://coveralls.io/repos/github/heathcliff26/cerberus-mergeguard/badge.svg)](https://coveralls.io/github/heathcliff26/cerberus-mergeguard)
[![Editorconfig Check](https://github.com/heathcliff26/cerberus-mergeguard/actions/workflows/editorconfig-check.yaml/badge.svg?event=push)](https://github.com/heathcliff26/cerberus-mergeguard/actions/workflows/editorconfig-check.yaml)
[![Generate test cover report](https://github.com/heathcliff26/cerberus-mergeguard/actions/workflows/coverprofiles.yaml/badge.svg)](https://github.com/heathcliff26/cerberus-mergeguard/actions/workflows/coverprofiles.yaml)
[![Renovate](https://github.com/heathcliff26/cerberus-mergeguard/actions/workflows/renovate.yaml/badge.svg)](https://github.com/heathcliff26/cerberus-mergeguard/actions/workflows/renovate.yaml)

# cerberus-mergeguard

Use cerberus bot to combine conditional workflows in big monorepos with required status checks.

Instead of having empty runs, crazy bash magic or other tricks, you only need to mark the "cerberus-mergeguard" status as required.

## Table of Contents

- [cerberus-mergeguard](#cerberus-mergeguard)
  - [Table of Contents](#table-of-contents)
  - [Usage](#usage)
    - [CLI Args](#cli-args)
    - [Image location](#image-location)
    - [Tags](#tags)
  - [Setup](#setup)
    - [Creating a github app](#creating-a-github-app)
    - [Installing your app](#installing-your-app)
    - [Running the bot](#running-the-bot)
    - [(Optional) Installing binary in CLI](#optional-installing-binary-in-cli)
  - [Credits](#credits)

## Usage

### CLI Args
```text
$ cerberus-mergeguard help
Guard PRs from merging until all triggered checks have passed

Usage: cerberus-mergeguard [OPTIONS] <COMMAND>

Commands:
  server   Run the bot and listen for webhook events on /webhook
  create   Create a new pending status check for a commit
  refresh  Refresh the state of the status check of a commit
  status   Check the status of a commit
  version  Print the version and exit
  help     Print this message or the help of the given subcommand(s)

Options:
      --log <LOG>        Log level to use, overrides the level given in the config file
  -c, --config <CONFIG>  Path to the config file [default: /config/config.yaml]
  -h, --help             Print help
```

### Image location

| Container Registry                                                                                       | Image                                        |
| -------------------------------------------------------------------------------------------------------- | -------------------------------------------- |
| [Github Container](https://github.com/users/heathcliff26/packages/container/package/cerberus-mergeguard) | `ghcr.io/heathcliff26/cerberus-mergeguard`   |
| [Docker Hub](https://hub.docker.com/r/heathcliff26/cerberus-mergeguard)                                  | `docker.io/heathcliff26/cerberus-mergeguard` |

### Tags

There are different flavors of the image:

| Tag(s)      | Description                                                 |
| ----------- | ----------------------------------------------------------- |
| **latest**  | Last released version of the image                          |
| **rolling** | Rolling update of the image, always build from main branch. |
| **vX.Y.Z**  | Released version of the image                               |

## Setup

### Creating a github app

In order to create and update status checks, github requires an app. To create a github app follow these steps:
1. Open github.com
2. Login to your account
3. Go to [Settings](https://github.com/settings/profile) -> [Developer Settings](https://github.com/settings/apps)
4. Under "Github Apps" click "New Github App"
5. Fill out all required fields
   - GitHub App name: The display name of your app, e.g. Cerberus Mergeguard
   - Homepage URL: URL to your Website
   - Webhook URL: The URL where your bot is running, e.g. <https://example.org/webhook>
   - Webhook Secret: Optional create a random string to enter here, to verify that webhook requests are sent by github
   - Permissions -> Repository permissions:
     - Checks: Read/Write
     - Issues: Read
     - Pull requests: Read
   - Events:
     - Check run
     - Issue comment
     - Pull request
6. After creating your app, go to your app -> "Private Keys" and generate a new key

### Installing your app

After you have created your app, navigate to it ([Settings](https://github.com/settings/profile) -> [Developer Settings](https://github.com/settings/apps) -> Your App).

In the "Installed App" tab install the app to your profile and select which repositories you want to use it for.

### Running the bot

Before you run the bot, copy both the [example configuration](examples/config.yaml) and your app private key to a folder.

Afterwards ensure you fill out all required attributes in the configuration file. The example has descriptions of the values.

Finally run the bot with
```bash
podman run -d -p 8080:8080 -v /path/to/config/:/config/ ghcr.io/heathcliff26/cerberus-mergeguard:latest
```

### (Optional) Installing binary in CLI

You can download the latest binary from the [releases](https://github.com/heathcliff26/cerberus-mergeguard/releases/latest) page.

Alternatively you can use cargo with `cargo install cerberus-mergeguard`.

## Credits

The avatar picture has been created with Google Gemini.
