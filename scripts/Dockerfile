FROM python:3.11-slim

WORKDIR /app

RUN pip install --no-cache-dir psycopg2-binary

COPY scripts/generate_data.py /app/generate_data.py

ENV DATABASE_URL=postgres://postgres:postgres@postgis:5432/ancient_city
ENV DYNASTY_FILTER=""
ENV SEED=42

CMD ["python", "-u", "generate_data.py"]
