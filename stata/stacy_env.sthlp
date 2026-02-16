{smcl}
{* *! version 0.1.0 - AUTO-GENERATED}{...}
{viewerjumpto "Syntax" "stacy_env##syntax"}{...}
{viewerjumpto "Description" "stacy_env##description"}{...}
{viewerjumpto "Options" "stacy_env##options"}{...}
{viewerjumpto "Returns" "stacy_env##returns"}{...}
{viewerjumpto "Examples" "stacy_env##examples"}{...}
{title:Title}

{phang}
{bf:stacy env} {hline 2} Show environment configuration


{marker syntax}{...}
{title:Syntax}

{p 8 17 2}
{cmd:stacy env} 

{marker description}{...}
{title:Description}

{pstd}
{cmd:stacy env} show environment configuration.


{marker returns}{...}
{title:Stored results}

{pstd}
{cmd:stacy env} stores the following in {cmd:r()}:

{synoptset 25 tabbed}{...}
{p2col 5 25 29 2: Scalars}{p_end}
{synopt:{cmd:r(adopath_count)}}Number of adopath entries{p_end}
{synopt:{cmd:r(has_config)}}stacy.toml exists (1=yes, 0=no){p_end}
{synopt:{cmd:r(show_progress)}}Progress shown (1=yes, 0=no){p_end}

{p2col 5 25 29 2: Macros}{p_end}
{synopt:{cmd:r(cache_dir)}}Global package cache directory{p_end}
{synopt:{cmd:r(log_dir)}}Project log directory{p_end}
{synopt:{cmd:r(project_root)}}Project root directory{p_end}
{synopt:{cmd:r(stata_binary)}}Path to Stata binary{p_end}
{synopt:{cmd:r(stata_source)}}How binary was detected{p_end}


{marker examples}{...}
{title:Examples}

{pstd}Basic usage:{p_end}
{phang2}{cmd:. stacy env}{p_end}


{marker author}{...}
{title:Author}

{pstd}
Jan Fasnacht{p_end}
{pstd}
{browse "https://github.com/janfasnacht/stacy":github.com/janfasnacht/stacy}{p_end}


{marker also_see}{...}
{title:Also see}

{psee}
{space 2}Help:  {helpb stacy}
{p_end}
