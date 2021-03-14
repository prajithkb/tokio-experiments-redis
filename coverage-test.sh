cargo cov -- report \
    $( \
      for file in \
        $( \
          RUSTFLAGS="-Zinstrument-coverage" \
            cargo test --tests --no-run --message-format=json \
              | jq -r "select(.profile.test == true) | .filenames[]" \
              | grep -v dSYM - \
        ); \
      do \
        printf "%s %s " -object $file; \
      done \
    ) \
  --instr-profile=json5format.profdata --summary-only --ignore-filename-regex='/.cargo/registry'
