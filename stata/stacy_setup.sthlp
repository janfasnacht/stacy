{smcl}
{* *! version 0.1.0 18jan2026}{...}
{viewerjumpto "Syntax" "stacy_setup##syntax"}{...}
{viewerjumpto "Description" "stacy_setup##description"}{...}
{viewerjumpto "Options" "stacy_setup##options"}{...}
{viewerjumpto "Returns" "stacy_setup##returns"}{...}
{viewerjumpto "Examples" "stacy_setup##examples"}{...}
{title:Title}

{phang}
{bf:stacy_setup} {hline 2} Download and install stacy binary


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy_setup} [{cmd:,} {it:options}]


{synoptset 20 tabbed}{...}
{synopthdr}
{synoptline}
{syntab:Main}
{synopt:{opt v:ersion(string)}}specific version to install (default: latest){p_end}
{synopt:{opt path(string)}}installation directory (default: ~/.local/bin){p_end}
{synopt:{opt force}}overwrite existing installation{p_end}
{synopt:{opt q:uietly}}suppress output{p_end}
{synoptline}


{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy_setup} downloads and installs the stacy binary from GitHub releases.
This is the first step in setting up stacy for use with Stata.

{pstd}
The command automatically detects your platform (macOS, Linux, Windows)
and architecture (x86_64, arm64) and downloads the appropriate binary.


{marker options}{...}
{title:Options}

{phang}
{opt version(string)} specifies a particular version to install. If not
provided, the latest version is installed.

{phang}
{opt path(string)} specifies the installation directory. Default is
{it:~/.local/bin} on Unix/macOS or {it:%LOCALAPPDATA%\stacy} on Windows.

{phang}
{opt force} overwrites an existing installation.

{phang}
{opt quietly} suppresses output.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy_setup} stores the following in {cmd:r()}:

{synoptset 20 tabbed}{...}
{p2col 5 20 24 2: Scalars}{p_end}
{synopt:{cmd:r(installed)}}1 if installed successfully, 0 otherwise{p_end}
{synopt:{cmd:r(already)}}1 if was already installed, 0 otherwise{p_end}

{p2col 5 20 24 2: Macros}{p_end}
{synopt:{cmd:r(version)}}version that was installed{p_end}
{synopt:{cmd:r(path)}}path where binary was installed{p_end}


{marker examples}{...}
{title:Examples}

{pstd}Install latest version:{p_end}
{phang2}{cmd:. stacy_setup}{p_end}

{pstd}Install specific version:{p_end}
{phang2}{cmd:. stacy_setup, version("v1.0.1")}{p_end}

{pstd}Install to custom location:{p_end}
{phang2}{cmd:. stacy_setup, path("/usr/local/bin")}{p_end}

{pstd}Reinstall (overwrite existing):{p_end}
{phang2}{cmd:. stacy_setup, force}{p_end}

{pstd}Check if installation succeeded:{p_end}
{phang2}{cmd:. stacy_setup}{p_end}
{phang2}{cmd:. if r(installed) == 1 {c -(}}{p_end}
{phang2}{cmd:.     display "Installed to: " r(path)}{p_end}
{phang2}{cmd:. {c )-}}{p_end}


{marker author}{...}
{title:Author}

{pstd}
Jan Fasnacht{p_end}
{pstd}
{browse "https://github.com/janfasnacht/stacy":github.com/janfasnacht/stacy}{p_end}


{marker also_see}{...}
{title:Also see}

{psee}
{space 2}Help:  {helpb stacy}, {helpb stacy_doctor}
{p_end}
