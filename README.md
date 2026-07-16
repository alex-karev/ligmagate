<a id="readme-top"></a>

<!-- PROJECT SHIELDS -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->
[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![License][license-shield]][license-url]

<!-- PROJECT LOGO -->
<br />
<div align="center">
  <!-- <a href="https://github.com/alex-karev/ollmo"> -->
  <!--   <img src="images/logo.png" alt="Logo" width="80" height="80"> -->
  <!-- </a> -->

  <h3 align="center">Ollmo: Open LLM Orchestrator</h3>

  <p align="center">
    OpenAI API-compatible LLM gateway for personal use.
    <!-- <br /> -->
    <!-- <a href="https://github.com/alex-karev/ollmo"><strong>Explore the docs »</strong></a> -->
    <!-- <br /> -->
    <br />
    &middot;
    <a href="https://github.com/alex-karev/ollmo/issues/new?labels=bug&template=bug-report---.md">Report Bug</a>
    &middot;
    <a href="https://github.com/alex-karev/ollmo/issues/new?labels=enhancement&template=feature-request---.md">Request Feature</a>
  </p>
</div>

<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li>
      <a href="#about-the-project">About The Project</a>
      <ul>
        <li><a href="#built-with">Built With</a></li>
      </ul>
    </li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#prerequisites">Prerequisites</a></li>
        <li><a href="#installation">Installation</a></li>
      </ul>
    </li>
    <li><a href="#usage">Usage</a></li>
    <li><a href="#roadmap">Roadmap</a></li>
    <li><a href="#contributing">Contributing</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#contact">Contact</a></li>
    <li><a href="#acknowledgments">Acknowledgments</a></li>
  </ol>
</details>

<!-- ABOUT THE PROJECT -->
## About The Project

<!-- [![Product Name Screen Shot][product-screenshot]](https://example.com) -->

With how fast LLMs are advancing, we now have hundreds of LLM-powered applications both FOSS and proprietary.
Most of them support using custom LLM API providers or custom system prompts.
Unfortunately, this customizability comes with a cost: you have to manage your configs yourself.
Each project has its unique format for customizing APIs, prompts and model parameters and switching from one provider with one model to another provider with another model would mean changing configs of all tools you use which adds a lot of friction.
This project aims to solve this exact problem by providing a self-hosted gateway server which combines all your LLM-related configurations.

Main features:

* Lightweight and written in Rust.
* Robust [config format](https://github.com/alex-karev/ollmo/blob/main/config.example.toml) allowing adding new providers, models and their customized variants easily.
* Supports replacing or combining system prompts for any LLM-powered tools you use.
* Does not depend on env variables. Supports storing keys in a separate env file.
* Self-hosted, can run as systemd service.
* Can be integrated with NixOS easily.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

### Built With

* [![Rust][Rust]][Rust-url]
* [![Tokio][Tokio]][Tokio-url]

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- GETTING STARTED -->
## Getting Started

This is an example of how you may give instructions on setting up your project locally.
To get a local copy up and running follow these simple example steps.

### Prerequisites

Install [rust compiler](https://rust-lang.org/tools/install/).

### Installation

NixOS:

1. Add this into your system flake inputs:
   ```nix
   ollmo = {
     url = "github:alex-karev/ollmo";
     inputs.nixpkgs.follows = "nixpkgs";
   };
   ```
2. Add module to your config:
   ```nix
   imports = [
     inputs.ollmo.nixosModules.ollmo
   ];
   ```
3. Enable the service somewhere in your system config:
   ```nix
   services.ollmo = {
       enable = true;
       # configPath = "/etc/ollmo";
   };
   ```

Other distros:

1. Clone the repo
   ```sh
   git clone https://github.com/alex-karev/ollmo.git
   ```
2. Build
   ```sh
   cargo build
   ```
3. Done! Binary will be in target. Copy it into your `/usr/bin`

*More distros will be supported natively in the future*

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- USAGE EXAMPLES -->
## Usage

By default, the config files are stored in `~/.config/ollmo` or in `/etc/ollmo`.
Config path can be customized with `--config` argument.

_Refer to this [config example](https://github.com/alex-karev/ollmo/blob/main/config.example.toml) for more details_

<p align="right">(<a href="#readme-top">back to top</a>)</p>


<!-- ROADMAP -->
## Roadmap

- [ ] Add Arch Linux and Debian builds.
- [ ] Add systemctl service template.
- [ ] Add docker build.
- [ ] Add text generation endpoint.
- [ ] Code cleanup and optimization.
- [ ] Add token counter.
- [ ] Add built-in providers like OpenAI and Deepseek.
- [ ] Add Claude support.

See the [open issues](https://github.com/alex-karev/ollmo/issues) for a full list of proposed features (and known issues).

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- CONTRIBUTING -->
## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

If you have a suggestion that would make this better, please fork the repo and create a pull request. You can also simply open an issue with the tag "enhancement".
Don't forget to give the project a star! Thanks again!

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

### Top contributors:

<a href="https://github.com/alex-karev/ollmo/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=alex-karev/ollmo" alt="contrib.rocks image" />
</a>

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- LICENSE -->
## License

Distributed under the GPL-3.0 License. See `LICENSE` for more information.

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- CONTACT -->
## Contact

Project Link: [https://github.com/alex-karev/ollmo](https://github.com/alex-karev/ollmo)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- ACKNOWLEDGMENTS -->
## Acknowledgments

* [Tensorzero](https://github.com/tensorzero/tensorzero) - for initial idea and config format inspiration
* [Best-README-Template](https://github.com/othneildrew/Best-README-Template) - for readme template

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->
[contributors-shield]: https://img.shields.io/github/contributors/alex-karev/ollmo.svg?style=for-the-badge
[contributors-url]: https://github.com/alex-karev/ollmo/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/alex-karev/ollmo.svg?style=for-the-badge
[forks-url]: https://github.com/alex-karev/ollmo/network/members
[stars-shield]: https://img.shields.io/github/stars/alex-karev/ollmo.svg?style=for-the-badge
[stars-url]: https://github.com/alex-karev/ollmo/stargazers
[issues-shield]: https://img.shields.io/github/issues/alex-karev/ollmo.svg?style=for-the-badge
[issues-url]: https://github.com/alex-karev/ollmo/issues
[license-shield]: https://img.shields.io/github/license/alex-karev/ollmo.svg?style=for-the-badge
[license-url]: https://github.com/alex-karev/ollmo/blob/master/LICENSE
[Rust]: https://img.shields.io/badge/rust-000000?style=for-the-badge&logo=rust&logoColor=white
[Rust-url]: https://rust-lang.org/
[Tokio]: https://img.shields.io/badge/tokio-000000?style=for-the-badge&logo=tokio&logoColor=white
[Tokio-url]: https://github.com/tokio-rs/tokio
