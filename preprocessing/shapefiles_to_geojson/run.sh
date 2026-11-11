#!/bin/bash

set -e

OUTPUT_DIR="../../data/raw_geojsons"
mkdir -p ${OUTPUT_DIR}

# Enter filename
# read -p "Enter file path to real_sitedata folder, e.g. (../../data/inputs):" shape_directory
shape_directory="../../data/inputs"

normalise_filename() {
    local name="$1"

    # Collapse double underscores to single
    local normalised="${name//__/_}"

    # Capitalise the first character after each underscore
    normalised=$(echo "$normalised" | perl -pe 's/_([a-z])/_\u$1/g')

    # Change capital F to lowercase f at the start of the filename
    if [[ "$normalised" == F* ]]; then
        normalised="f${normalised:1}"
    fi

    echo "${normalised}.geojson"
}

find "$shape_directory" -type f -name "*.shp" | while read -r file; do
    filename=$(basename "$file" .shp)
    case "$filename" in
        Larger_LOE_heathrow|Larger_LOE_smaller)
            echo "Skipping ${filename}.shp"
            continue
            ;;
    esac

    output_file=$(normalise_filename "$filename")

    python -c "import geopandas as gpd; gpd.read_file('${file}').to_file('${OUTPUT_DIR}/${output_file}', driver='GeoJSON')"
done